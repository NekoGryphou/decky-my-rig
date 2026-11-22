using System.Buffers;
using System.Text;
using System.Text.Json;
using MyRigConfig.Core.Shared;

using static MyRigConfig.Core.LocalControl.LocalControlConstants;

namespace MyRigConfig.Core.LocalControl;

// ! The pipe transport authenticates the server before any of these messages are sent.
internal static class LocalControlProtocol
{
    internal static bool IsSettablePort(int port) => port is >= MinSettablePort and <= MaxPort;

    internal static byte[] StatusRequest() => Bounded(JsonSerializer.SerializeToUtf8Bytes(
        new StatusRequest(1, "status"), LocalControlJson.Default.StatusRequest));
    internal static byte[] ListRequest() => Bounded(JsonSerializer.SerializeToUtf8Bytes(
        new ListRequest(1, "list"), LocalControlJson.Default.ListRequest));
    internal static byte[] RemoveRequest(Guid hostId, Guid deviceId) => Bounded(JsonSerializer.SerializeToUtf8Bytes(
        new RemoveRequest(1, "remove", hostId.ToString("D"), deviceId.ToString("D")), LocalControlJson.Default.RemoveRequest));
    internal static byte[] RenameRequest(Guid hostId, Guid deviceId, string name)
    {
        ValidateName(name);
        return Bounded(JsonSerializer.SerializeToUtf8Bytes(
            new RenameRequest(1, "rename", hostId.ToString("D"), deviceId.ToString("D"), name), LocalControlJson.Default.RenameRequest));
    }
    internal static byte[] SetPortRequest(Guid hostId, int port)
    {
        if (!IsSettablePort(port)) throw Invalid();
        return Bounded(JsonSerializer.SerializeToUtf8Bytes(
            new SetPortRequest(1, "set_port", hostId.ToString("D"), port), LocalControlJson.Default.SetPortRequest));
    }
    internal static byte[] SetTestModeRequest(Guid hostId, bool enabled) => Bounded(JsonSerializer.SerializeToUtf8Bytes(
        new SetTestModeRequest(1, "set_test_mode", hostId.ToString("D"), enabled), LocalControlJson.Default.SetTestModeRequest));
    internal static byte[] OpenPairingRequest(Guid hostId, string name)
    {
        ValidateName(name);
        return Bounded(JsonSerializer.SerializeToUtf8Bytes(
            new OpenPairingRequest(1, "open_pairing", hostId.ToString("D"), name), LocalControlJson.Default.OpenPairingRequest));
    }

    internal static byte[] ClosePairingRequest(Guid hostId) => Bounded(JsonSerializer.SerializeToUtf8Bytes(
        new ClosePairingRequest(1, "close_pairing", hostId.ToString("D")), LocalControlJson.Default.ClosePairingRequest));

    private static byte[] Bounded(byte[] bytes)
    {
        if (bytes.Length is < 1 or > RequestLimit) throw Invalid();
        return bytes;
    }

    internal static LocalControlReply ParseReply(ReadOnlyMemory<byte> bytes,
        LocalControlOperation operation, Guid? expectedHost = null)
    {
        if (bytes.Length is < 1 or > ResponseLimit || !Enum.IsDefined(operation)) throw Invalid();
        try
        {
            StrictUtf8.GetCharCount(bytes.Span);
            using var document = JsonDocument.Parse(bytes, new JsonDocumentOptions { MaxDepth = JsonMaxDepth, AllowDuplicateProperties = false });
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object ||
                !root.TryGetProperty("ok", out var ok) || ok.ValueKind is not (JsonValueKind.True or JsonValueKind.False))
                throw Invalid();
            LocalControlReply result;
            if (!ok.GetBoolean())
            {
                var reply = Deserialize(bytes, LocalControlJson.Default.ErrorReply);
                if (reply.Version != 1 || reply.Ok || !Errors.Contains(reply.Error)) throw Invalid();
                result = new LocalControlErrorReply(reply.Error);
            }
            else
            {
                result = operation switch
                {
                    LocalControlOperation.Status => ParseStatus(Deserialize(bytes, LocalControlJson.Default.StatusReply)),
                    LocalControlOperation.List => ParseList(Deserialize(bytes, LocalControlJson.Default.ListReply)),
                    LocalControlOperation.Remove => ParseRemove(Deserialize(bytes, LocalControlJson.Default.RemoveReply)),
                    LocalControlOperation.Rename => ParseRename(Deserialize(bytes, LocalControlJson.Default.RenameReply)),
                    LocalControlOperation.OpenPairing => ParsePairing(Deserialize(bytes, LocalControlJson.Default.PairingReply)),
                    LocalControlOperation.SetPort => ParsePort(Deserialize(bytes, LocalControlJson.Default.SetPortReply)),
                    LocalControlOperation.ClosePairing => ParseClosePairing(Deserialize(bytes, LocalControlJson.Default.ClosePairingReply)),
                    LocalControlOperation.SetTestMode => ParseTestMode(Deserialize(bytes, LocalControlJson.Default.SetTestModeReply)),
                    _ => throw Invalid()
                };
            }
            if (expectedHost.HasValue && result is LocalControlHostReply hostReply && hostReply.HostId != expectedHost.Value)
                throw Invalid();
            return result;
        }
        catch (DecoderFallbackException) { throw Invalid(); }
        catch (InvalidOperationException) { throw Invalid(); }
        catch (JsonException) { throw Invalid(); }
    }

    private static T Deserialize<T>(ReadOnlyMemory<byte> bytes, System.Text.Json.Serialization.Metadata.JsonTypeInfo<T> type) =>
        JsonSerializer.Deserialize(bytes.Span, type) ?? throw Invalid();

    private static LocalControlReply ParseStatus(StatusReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        if (reply.Port is < 1 or > MaxPort || reply.ServiceVersion is null or { Length: < 1 or > 64 } ||
            reply.ServiceVersion.Any(c => !char.IsAsciiLetterOrDigit(c) && c is not ('.' or '-' or '+')) ||
            reply.ListenerState is not ("listening" or "unavailable")) throw Invalid();
        return new LocalControlStatusReply(Identity(reply.HostId), reply.Port, reply.TestMode, reply.ServiceVersion, reply.ListenerState);
    }
    private static LocalControlReply ParseList(ListReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        if (reply.PairedDevices is null || reply.PairedDevices.Count > LocalControlConstants.MaxPairedDevices) throw Invalid();
        var ids = new HashSet<Guid>();
        var devices = new List<PairedDeviceSummary>();
        foreach (var item in reply.PairedDevices)
        {
            if (item is null) throw Invalid();
            var id = Identity(item.DeviceId);
            if (!ids.Add(id)) throw Invalid();
            ValidateName(item.Name);
            devices.Add(new(id, item.Name));
        }
        return new LocalControlListReply(Identity(reply.HostId), devices.AsReadOnly());
    }
    private static LocalControlReply ParseRename(RenameReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        return new LocalControlRenameReply(Identity(reply.HostId), reply.Renamed);
    }
    private static LocalControlReply ParseRemove(RemoveReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        return new LocalControlRemoveReply(Identity(reply.HostId), reply.Removed);
    }
    private static LocalControlReply ParsePort(SetPortReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        if (!IsSettablePort(reply.Port)) throw Invalid();
        return new LocalControlPortReply(Identity(reply.HostId), reply.Port);
    }
    private static LocalControlReply ParseTestMode(SetTestModeReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        return new LocalControlTestModeReply(Identity(reply.HostId), reply.TestMode);
    }
    private static LocalControlReply ParsePairing(PairingReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        if (reply.Code is null || reply.Code.Length != 8 || reply.Code.Any(c => c is < '0' or > '9') ||
            reply.ExpiresInSeconds != PairingProtocolConstants.LifetimeSeconds) throw Invalid();
        return new LocalControlPairingReply(Identity(reply.HostId), reply.Code);
    }
    private static LocalControlReply ParseClosePairing(ClosePairingReply reply)
    {
        ValidEnvelope(reply.Version, reply.Ok);
        return new LocalControlClosePairingReply(Identity(reply.HostId));
    }
    private static void ValidEnvelope(int version, bool ok)
    { if (version != 1 || !ok) throw Invalid(); }
    private static Guid Identity(string text)
    {
        if (!Guid.TryParseExact(text, "D", out var id) || !string.Equals(id.ToString("D"), text, StringComparison.Ordinal)) throw Invalid();
        return id;
    }
    private static JsonException Invalid() => new("Invalid local control message.");
    private static void ValidateName(string name)
    {
        if (!IsValidName(name)) throw Invalid();
    }

    /// <summary>The service's rule: trimmed, 1–80 characters, no control characters and nothing in
    /// <see cref="LocalControlConstants.InvisibleRanges"/>.</summary>
    internal static bool IsValidName(string? name)
    {
        if (name is null || !string.Equals(name.Trim(), name, StringComparison.Ordinal)) return false;
        var span = name.AsSpan();
        var count = 0;
        while (!span.IsEmpty)
        {
            if (Rune.DecodeFromUtf16(span, out var rune, out var consumed) != OperationStatus.Done
                || Rune.IsControl(rune) || IsInvisible(rune.Value)) return false;
            span = span[consumed..];
            if (++count > 80) return false;
        }
        return count > 0;
    }

    private static bool IsInvisible(int value)
    {
        foreach (var (first, last) in LocalControlConstants.InvisibleRanges)
            if (value >= first && value <= last) return true;
        return false;
    }
}
