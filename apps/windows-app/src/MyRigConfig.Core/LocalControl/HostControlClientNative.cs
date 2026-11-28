using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using Microsoft.Win32.SafeHandles;
using Windows.Win32;
using Windows.Win32.Foundation;
using Windows.Win32.Storage.FileSystem;

using static MyRigConfig.Core.LocalControl.LocalControlConstants;

namespace MyRigConfig.Core.LocalControl;

// - The part of the client that talks to the real pipe; the framing lives in HostControlClientService.
[SupportedOSPlatform("windows10.0")]
internal sealed partial class HostControlClientService
{
    private static async Task<byte[]> ExchangeNativeAsync(byte[] request, Action onSending,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var pipe = Connect();
        using var identity = ControlServerIdentityNative.Verify(pipe);
        return await ExchangeFrameAsync(pipe, request, () => identity.Recheck(pipe),
            onSending, cancellationToken).ConfigureAwait(false);
    }

    private static NamedPipeClientStream Connect()
    {
        if (!PInvoke.WaitNamedPipe(PipePath, PipeWaitMilliseconds)) throw new HostControlException(false);
        // ! These exact client rights leave out FILE_CREATE_PIPE_INSTANCE, DELETE, WRITE_DAC and WRITE_OWNER.
        // ! Identification level only allows checking identity.
        const uint rights = (uint)(FILE_ACCESS_RIGHTS.FILE_GENERIC_READ | FILE_ACCESS_RIGHTS.FILE_GENERIC_WRITE)
            & ~(uint)FILE_ACCESS_RIGHTS.FILE_CREATE_PIPE_INSTANCE;
        using var file = PInvoke.CreateFile(PipePath, rights, FILE_SHARE_MODE.FILE_SHARE_NONE, null,
            FILE_CREATION_DISPOSITION.OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES.FILE_FLAG_OVERLAPPED | FILE_FLAGS_AND_ATTRIBUTES.SECURITY_SQOS_PRESENT
                | FILE_FLAGS_AND_ATTRIBUTES.SECURITY_IDENTIFICATION, null!);
        if (file.IsInvalid)
            throw new HostControlException(false,
                unauthorized: (WIN32_ERROR)Marshal.GetLastPInvokeError() == WIN32_ERROR.ERROR_ACCESS_DENIED);
        var handle = new SafePipeHandle(file.DangerousGetHandle(), ownsHandle: true);
        file.SetHandleAsInvalid(); // - The pipe stream owns the handle from here.
        try { return new NamedPipeClientStream(PipeDirection.InOut, true, true, handle); }
        catch { handle.Dispose(); throw; }
    }
}
