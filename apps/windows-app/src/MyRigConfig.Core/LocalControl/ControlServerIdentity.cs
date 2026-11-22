using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using System.Security;
using System.Security.Principal;
using Microsoft.Win32.SafeHandles;
using static MyRigConfig.Core.LocalControl.ControlServerIdentityConstants;
using Windows.Win32;
using Windows.Win32.Foundation;
using Windows.Win32.Security;
using Windows.Win32.Security.Authorization;
using Windows.Win32.System.Services;

namespace MyRigConfig.Core.LocalControl;

// ! The service SID owns its pipe. Checking the service's PID with the SCM before and after the
//   exchange also catches a restart, without needing access to its process token.
[SupportedOSPlatform("windows10.0")]
internal sealed partial class ControlServerIdentity : IDisposable
{
    private readonly CloseServiceHandleSafeHandle service;
    private readonly uint pid;

    private ControlServerIdentity(CloseServiceHandleSafeHandle service, uint pid)
    { this.service = service; this.pid = pid; }

    internal static ControlServerIdentity Verify(NamedPipeClientStream pipe)
    {
        using var manager = PInvoke.OpenSCManager(null!, null!, ServiceManagerConnect);
        if (manager.IsInvalid) throw Unavailable();
        var serviceHandle = PInvoke.OpenService(manager, ServiceName, ServiceQueryStatus);
        try
        {
            if (serviceHandle.IsInvalid) throw Unavailable();
            var servicePid = ServicePid(serviceHandle);
            if (!PInvoke.GetNamedPipeServerProcessId(pipe.SafePipeHandle, out var serverPid) || serverPid != servicePid)
                throw Unavailable();
            RequireOwner(pipe.SafePipeHandle, ServiceSid());
            // ! The exchange checks the process again before its first byte and after the reply.
            return new ControlServerIdentity(serviceHandle, servicePid);
        }
        catch (IdentityNotMappedException)
        {
            serviceHandle.Dispose();
            throw Unavailable();
        }
        catch
        {
            serviceHandle.Dispose();
            throw;
        }
    }

    private static SecurityIdentifier? serviceSid;

    // ? The service SID is fixed by its name. Only a successful lookup is kept, so a service
    //   installed later is still found.
    private static SecurityIdentifier ServiceSid() => serviceSid ??=
        (SecurityIdentifier)new NTAccount("NT SERVICE", ServiceName).Translate(typeof(SecurityIdentifier));

    /// <summary>ID of the running service's process, read with status rights only, or null when it
    /// isn't running.</summary>
    internal static int? RunningProcessId()
    {
        using var manager = PInvoke.OpenSCManager(null!, null!, ServiceManagerConnect);
        if (manager.IsInvalid) return null;
        using var serviceHandle = PInvoke.OpenService(manager, ServiceName, ServiceQueryStatus);
        if (serviceHandle.IsInvalid) return null;
        try { return (int)ServicePid(serviceHandle); }
        catch (SecurityException) { return null; }
    }

    internal void Recheck(NamedPipeClientStream pipe)
    {
        if (ServicePid(service) != pid
            || !PInvoke.GetNamedPipeServerProcessId(pipe.SafePipeHandle, out var serverPid) || serverPid != pid)
            throw Unavailable();
    }

    private static uint ServicePid(CloseServiceHandleSafeHandle service)
    {
        Span<byte> buffer = stackalloc byte[Marshal.SizeOf<ControlServiceStatus>()];
        if (!PInvoke.QueryServiceStatusEx(service, (SC_STATUS_TYPE)0, buffer, out _)) throw Unavailable();
        var status = MemoryMarshal.Read<ControlServiceStatus>(buffer);
        if (status.ServiceType != 0x10 || status.CurrentState != 4 || status.ProcessId == 0)
            throw Unavailable(); // exact OWN_PROCESS and RUNNING, not pending/shared
        return status.ProcessId;
    }

    internal static unsafe void RequireOwner(SafePipeHandle pipe, SecurityIdentifier expected)
    {
        PSID owner = default;
        PSECURITY_DESCRIPTOR descriptor = default;
        var added = false;
        try
        {
            pipe.DangerousAddRef(ref added);
            if (PInvoke.GetSecurityInfo(new HANDLE(pipe.DangerousGetHandle()), SE_OBJECT_TYPE.SE_KERNEL_OBJECT,
                OBJECT_SECURITY_INFORMATION.OWNER_SECURITY_INFORMATION, &owner, null, null, null, &descriptor) != WIN32_ERROR.ERROR_SUCCESS
                || owner.Value == null)
                throw Unavailable();
            if (!expected.Equals(new SecurityIdentifier((IntPtr)owner.Value))) throw Unavailable();
        }
        finally
        {
            if (descriptor.Value != null) PInvoke.LocalFree(new HLOCAL((IntPtr)descriptor.Value));
            if (added) pipe.DangerousRelease();
        }
    }

    private static SecurityException Unavailable() => new(ControlServerIdentityMessages.Unavailable);
    public void Dispose() => service.Dispose();
}
