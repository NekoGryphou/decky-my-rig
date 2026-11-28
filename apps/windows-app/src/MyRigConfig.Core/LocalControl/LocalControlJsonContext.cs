using System.Text.Json;
using System.Text.Json.Serialization;

using static MyRigConfig.Core.LocalControl.LocalControlConstants;

namespace MyRigConfig.Core.LocalControl;

[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
    RespectRequiredConstructorParameters = true,
    RespectNullableAnnotations = true,
    AllowDuplicateProperties = false,
    MaxDepth = JsonMaxDepth)]
[JsonSerializable(typeof(StatusRequest))]
[JsonSerializable(typeof(ListRequest))]
[JsonSerializable(typeof(RemoveRequest))]
[JsonSerializable(typeof(RenameRequest))]
[JsonSerializable(typeof(SetPortRequest))]
[JsonSerializable(typeof(SetTestModeRequest))]
[JsonSerializable(typeof(OpenPairingRequest))]
[JsonSerializable(typeof(ClosePairingRequest))]
[JsonSerializable(typeof(ClosePairingReply))]
[JsonSerializable(typeof(ErrorReply))]
[JsonSerializable(typeof(StatusReply))]
[JsonSerializable(typeof(ListReply))]
[JsonSerializable(typeof(RemoveReply))]
[JsonSerializable(typeof(RenameReply))]
[JsonSerializable(typeof(SetPortReply))]
[JsonSerializable(typeof(SetTestModeReply))]
[JsonSerializable(typeof(PairingReply))]
internal partial class LocalControlJsonContext : JsonSerializerContext;
