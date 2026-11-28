using System.Buffers.Binary;
using System.Runtime.Versioning;
using System.Security;
using System.Security.Principal;
using System.Text.Json;

using static MyRigConfig.Core.LocalControl.LocalControlConstants;

namespace MyRigConfig.Core.LocalControl;

[SupportedOSPlatform("windows10.0")]
internal sealed partial class HostControlClientService
{
    private readonly Func<byte[], Action, CancellationToken, Task<byte[]>> exchange;

    internal HostControlClientService() : this(ExchangeNativeAsync) { }

    internal HostControlClientService(Func<byte[], Action, CancellationToken, Task<byte[]>> exchange)
    {
        this.exchange = exchange;
    }

    internal Task<LocalControlReply> StatusAsync(CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.StatusRequest(), LocalControlOperation.Status, null, cancellationToken);
    internal Task<LocalControlReply> ListAsync(CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.ListRequest(), LocalControlOperation.List, null, cancellationToken);
    internal Task<LocalControlReply> RemoveAsync(Guid hostId, Guid deviceId, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.RemoveRequest(hostId, deviceId), LocalControlOperation.Remove, hostId, cancellationToken);
    internal Task<LocalControlReply> RenameAsync(Guid hostId, Guid deviceId, string name, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.RenameRequest(hostId, deviceId, name), LocalControlOperation.Rename, hostId, cancellationToken);
    internal Task<LocalControlReply> SetPortAsync(Guid hostId, int port, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.SetPortRequest(hostId, port), LocalControlOperation.SetPort, hostId, cancellationToken);
    internal Task<LocalControlReply> SetTestModeAsync(Guid hostId, bool enabled, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.SetTestModeRequest(hostId, enabled), LocalControlOperation.SetTestMode, hostId, cancellationToken);
    internal Task<LocalControlReply> OpenPairingAsync(Guid hostId, string name, Action onSending, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.OpenPairingRequest(hostId, name), LocalControlOperation.OpenPairing, hostId, cancellationToken, onSending);

    internal Task<LocalControlReply> ClosePairingAsync(Guid hostId, CancellationToken cancellationToken) =>
        ExchangeAsync(LocalControlProtocolService.ClosePairingRequest(hostId), LocalControlOperation.ClosePairing, hostId, cancellationToken);

    private async Task<LocalControlReply> ExchangeAsync(byte[] request, LocalControlOperation operation,
        Guid? expectedHost, CancellationToken cancellationToken, Action? onSending = null)
    {
        var started = false;
        try
        {
            using var deadline = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            deadline.CancelAfter(ExchangeDeadline);
            // ! The native connection and identity checks can't be force- canceled, so the task is awaited even after
            // ! cancellation.
            var reply = await Task.Run(() => exchange(request, () => { started = true; onSending?.Invoke(); }, deadline.Token)).ConfigureAwait(false);
            return LocalControlProtocolService.ParseReply(reply, operation, expectedHost);
        }
        // * Only what a pipe, the identity check or a timeout can throw means "host unavailable". A programming error
        // * keeps its type so the app's handler logs it.
        catch (Exception exception) when (exception is IOException or SecurityException or
            UnauthorizedAccessException or System.ComponentModel.Win32Exception or JsonException or
            TimeoutException or OperationCanceledException or IdentityNotMappedException or HostControlException)
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

    // ? Internal so the framing and admission tests can call it directly. In production it gets the connected pipe and
    // ? the native verifier.
    internal static async Task<byte[]> ExchangeFrameAsync(Stream pipe, byte[] request,
        Action verify, Action onSending, CancellationToken cancellationToken)
    {
        if (request.Length is < 1 or > RequestLimit) throw new HostControlException(false);
        cancellationToken.ThrowIfCancellationRequested();
        verify(); // ! No frame prefix or application bytes go out before authentication.
        cancellationToken.ThrowIfCancellationRequested();
        var frame = new byte[FramePrefixBytes + request.Length];
        BinaryPrimitives.WriteUInt32LittleEndian(frame, (uint)request.Length);
        request.CopyTo(frame, FramePrefixBytes);
        onSending(); // ! A partial prefix or write failure leaves the outcome uncertain.
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
