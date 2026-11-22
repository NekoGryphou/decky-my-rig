using System.Buffers.Binary;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using System.Security;
using System.Security.Principal;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using Windows.Win32;
using Windows.Win32.Foundation;
using Windows.Win32.Storage.FileSystem;

using static MyRigConfig.Core.LocalControl.LocalControlConstants;

namespace MyRigConfig.Core.LocalControl;

[SupportedOSPlatform("windows10.0")]
internal sealed partial class HostControlClient
{
    private readonly Func<byte[], Action, CancellationToken, Task<byte[]>> exchange;

    internal HostControlClient() : this(ExchangeNativeAsync) { }

    internal HostControlClient(Func<byte[], Action, CancellationToken, Task<byte[]>> exchange)
    {
        this.exchange = exchange;
    }

    internal Task<LocalControlReply> StatusAsync(CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.StatusRequest(), LocalControlOperation.Status, null, cancellationToken);
    internal Task<LocalControlReply> ListAsync(CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.ListRequest(), LocalControlOperation.List, null, cancellationToken);
    internal Task<LocalControlReply> RemoveAsync(Guid hostId, Guid deviceId, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.RemoveRequest(hostId, deviceId), LocalControlOperation.Remove, hostId, cancellationToken);
    internal Task<LocalControlReply> RenameAsync(Guid hostId, Guid deviceId, string name, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.RenameRequest(hostId, deviceId, name), LocalControlOperation.Rename, hostId, cancellationToken);
    internal Task<LocalControlReply> SetPortAsync(Guid hostId, int port, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.SetPortRequest(hostId, port), LocalControlOperation.SetPort, hostId, cancellationToken);
    internal Task<LocalControlReply> SetTestModeAsync(Guid hostId, bool enabled, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.SetTestModeRequest(hostId, enabled), LocalControlOperation.SetTestMode, hostId, cancellationToken);
    internal Task<LocalControlReply> OpenPairingAsync(Guid hostId, string name, Action onSending, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.OpenPairingRequest(hostId, name), LocalControlOperation.OpenPairing, hostId, cancellationToken, onSending);

    internal Task<LocalControlReply> ClosePairingAsync(Guid hostId, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocol.ClosePairingRequest(hostId), LocalControlOperation.ClosePairing, hostId, cancellationToken);

    private async Task<LocalControlReply> ExchangeAsync(byte[] request, LocalControlOperation operation,
        Guid? expectedHost, CancellationToken cancellationToken, Action? onSending = null)
    {
        var started = false;
        try
        {
            using var deadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            deadline.CancelAfter(ExchangeDeadline);
            // ! The native connection and identity checks can't be force-canceled, so the task is
            //   awaited even after cancellation.
            var reply = await Task.Run(() => exchange(request, () => { started = true; onSending?.Invoke(); }, deadline.Token)).ConfigureAwait(false);
            return LocalControlProtocol.ParseReply(reply, operation, expectedHost);
        }
        catch (Exception exception) when (exception is IOException or SecurityException or
            UnauthorizedAccessException or System.ComponentModel.Win32Exception or JsonException or
            OperationCanceledException or IdentityNotMappedException or HostControlException)
        {
            throw Failure(exception, started, operation, cancellationToken);
        }
    }

    /// <summary>The caller's own cancellation stays a cancellation, so shutting down never shows up
    /// as a host failure. Anything else is a host failure.</summary>
    internal static Exception Failure(Exception exception, bool started, LocalControlOperation operation,
        CancellationToken cancellationToken) =>
        cancellationToken.IsCancellationRequested
            ? new OperationCanceledException(cancellationToken)
            : exception as HostControlException
                ?? new HostControlException(started && operation != LocalControlOperation.List);

    private static async Task<byte[]> ExchangeNativeAsync(byte[] request, Action onSending,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var pipe = Connect();
        using var identity = ControlServerIdentity.Verify(pipe);
        return await ExchangeFrameAsync(pipe, request, () => identity.Recheck(pipe),
            onSending, cancellationToken).ConfigureAwait(false);
    }

    private static NamedPipeClientStream Connect()
    {
        if (!PInvoke.WaitNamedPipe(PipePath, PipeWaitMilliseconds)) throw new HostControlException(false);
        // ! These exact client rights leave out FILE_CREATE_PIPE_INSTANCE, DELETE, WRITE_DAC and
        //   WRITE_OWNER. Identification level only allows checking identity.
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
        file.SetHandleAsInvalid(); // transfer ownership to the pipe stream
        try { return new NamedPipeClientStream(PipeDirection.InOut, true, true, handle); }
        catch { handle.Dispose(); throw; }
    }

    // ? Internal so the framing and admission tests can call it directly. In production it gets the
    //   connected pipe and the native verifier.
    internal static async Task<byte[]> ExchangeFrameAsync(Stream pipe, byte[] request,
        Action verify, Action onSending, CancellationToken cancellationToken)
    {
        if (request.Length is < 1 or > RequestLimit) throw new HostControlException(false);
        cancellationToken.ThrowIfCancellationRequested();
        verify(); // no frame prefix or application bytes before authentication
        cancellationToken.ThrowIfCancellationRequested();
        var frame = new byte[FramePrefixBytes + request.Length];
        BinaryPrimitives.WriteUInt32LittleEndian(frame, (uint)request.Length);
        request.CopyTo(frame, FramePrefixBytes);
        onSending(); // partial prefix/write failure is an uncertain mutation
        await pipe.WriteAsync(frame, cancellationToken).ConfigureAwait(false);
        var prefix = new byte[FramePrefixBytes];
        await pipe.ReadExactlyAsync(prefix, cancellationToken).ConfigureAwait(false);
        var length = BinaryPrimitives.ReadUInt32LittleEndian(prefix);
        if (length is < 1 or > ResponseLimit) throw new IOException("Invalid control response.");
        var response = new byte[(int)length];
        await pipe.ReadExactlyAsync(response, cancellationToken).ConfigureAwait(false);
        verify();
        return response;
    }
}
