using MyRigConfig.Shared;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Features.Host;

// - The shared steps of removing and renaming a paired device.
public partial class HostViewModel
{
    private async Task<bool?> RemovedAsync(Guid hostId, Guid deviceId, CancellationToken cancellationToken) =>
        await client.RemoveAsync(hostId, deviceId, cancellationToken) is LocalControlRemoveReply reply ? reply.Removed : null;

    private async Task<bool?> RenamedAsync(Guid hostId, Guid deviceId, string newName, CancellationToken cancellationToken) =>
        await client.RenameAsync(hostId, deviceId, newName, cancellationToken) is LocalControlRenameReply reply ? reply.Renamed : null;

    /// <param name="send">True when the change happened, false when the device is no longer
    /// paired, null for a reply of another kind.</param>
    private async Task ChangeDeviceAsync(Guid hostId, Guid deviceId, string name, Func<CancellationToken, Task<bool?>> send,
        string success, string uncertain, CancellationToken cancellationToken)
    {
        if (!await StillListedAsync(hostId, deviceId, name, cancellationToken)) return;
        var (message, severity) = (uncertain, MessageTone.Warning);
        try
        {
            if (await send(cancellationToken) is { } done)
                (message, severity) = done
                    ? (success, MessageTone.Success)
                    : (HostMessages.NoLongerPaired, MessageTone.Informational);
        }
        catch (HostControlException) { }
        // ! Don't update the list from a guess; show what the service reports.
        await RefreshManagementAsync(cancellationToken);
        ShowFeedback(message, severity);
    }
}
