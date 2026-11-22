using System.Text.Json.Serialization;
using MyRigConfig.Core.Shared;

namespace MyRigConfig.Core.LocalControl;

internal sealed record StatusRequest(int Version, string Operation);
internal sealed record ListRequest(int Version, string Operation);
internal sealed record RemoveRequest(int Version, string Operation, string HostId, string DeviceId);
internal sealed record RenameRequest(int Version, string Operation, string HostId, string DeviceId,
    [property: JsonConverter(typeof(UnicodeNameConverter))] string Name);
internal sealed record SetPortRequest(int Version, string Operation, string HostId, int Port);
internal sealed record SetTestModeRequest(int Version, string Operation, string HostId, bool Enabled);
internal sealed record OpenPairingRequest(int Version, string Operation, string HostId,
    [property: JsonConverter(typeof(UnicodeNameConverter))] string Name);

internal sealed record ClosePairingRequest(int Version, string Operation, string HostId);
internal sealed record ClosePairingReply(int Version, bool Ok, string HostId);

internal sealed record ErrorReply(int Version, bool Ok, string Error);
internal sealed record StatusReply(int Version, bool Ok, string HostId, int Port, bool TestMode, string ServiceVersion, string ListenerState);
internal sealed record ListReply(int Version, bool Ok, string HostId, List<DeviceMessage> PairedDevices);
internal sealed record DeviceMessage(string DeviceId, string Name);
internal sealed record RemoveReply(int Version, bool Ok, string HostId, bool Removed);
internal sealed record RenameReply(int Version, bool Ok, string HostId, bool Renamed);
internal sealed record SetPortReply(int Version, bool Ok, string HostId, int Port);
internal sealed record SetTestModeReply(int Version, bool Ok, string HostId, bool TestMode);
internal sealed record PairingReply(int Version, bool Ok, string HostId, string Code, int ExpiresInSeconds);

internal enum LocalControlOperation { Status, List, Remove, Rename, OpenPairing, SetPort, SetTestMode, ClosePairing }

internal abstract record LocalControlReply;
/// <summary>A success reply. The host ID is only used for routing, never shown in the UI.</summary>
internal abstract record LocalControlHostReply(Guid HostId) : LocalControlReply;
internal sealed record LocalControlListReply(Guid HostId, IReadOnlyList<PairedDeviceSummary> PairedDevices) : LocalControlHostReply(HostId);
internal sealed record LocalControlRemoveReply(Guid HostId, bool Removed) : LocalControlHostReply(HostId);
internal sealed record LocalControlRenameReply(Guid HostId, bool Renamed) : LocalControlHostReply(HostId);
internal sealed record LocalControlPortReply(Guid HostId, int Port) : LocalControlHostReply(HostId);
internal sealed record LocalControlStatusReply(Guid HostId, int Port, bool TestMode, string ServiceVersion, string ListenerState)
    : LocalControlHostReply(HostId);
internal sealed record LocalControlTestModeReply(Guid HostId, bool TestMode) : LocalControlHostReply(HostId);
internal sealed record LocalControlClosePairingReply(Guid HostId) : LocalControlHostReply(HostId);
internal sealed record LocalControlErrorReply(string Error) : LocalControlReply;
