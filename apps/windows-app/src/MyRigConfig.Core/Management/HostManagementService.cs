using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Shared;

namespace MyRigConfig.Core.Management;

// ! Routing only comes from the local list, read after the service is authenticated.
internal sealed class HostManagementService
{
    private Guid? hostId;
    private IReadOnlyList<PairedDeviceSummary> devices = [];
    private readonly SemaphoreSlim admission = new(1, 1);
    internal bool TryBegin() => admission.Wait(0);
    internal Task BeginAsync(CancellationToken cancellationToken) => admission.WaitAsync(cancellationToken);
    internal void End() => admission.Release();
    internal bool Available => hostId.HasValue;
    internal IReadOnlyList<PairedDeviceSummary> Devices => devices;
    internal void Publish(LocalControlListReply reply)
    {
        hostId = reply.HostId;
        devices = reply.PairedDevices;
    }
    internal void Unavailable() { hostId = null; devices = []; }
    internal Guid RequireHost() => hostId ?? throw new InvalidOperationException("Host control is unavailable.");
    internal bool MatchesHost(Guid confirmedHost) => hostId == confirmedHost;
    internal bool MatchesDevice(Guid confirmedHost, Guid confirmedDevice, string confirmedName) =>
        MatchesHost(confirmedHost) && devices.Any(device => device.DeviceId == confirmedDevice &&
            string.Equals(device.Name, confirmedName, StringComparison.Ordinal));
}
