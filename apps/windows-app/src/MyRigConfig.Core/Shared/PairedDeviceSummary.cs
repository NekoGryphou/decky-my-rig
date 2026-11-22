namespace MyRigConfig.Core.Shared;

// ! The host identity and the credential digest stay out of UI objects.
public sealed record PairedDeviceSummary(Guid DeviceId, string Name);
