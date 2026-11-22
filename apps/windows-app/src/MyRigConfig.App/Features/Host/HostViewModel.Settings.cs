using MyRigConfig.Shared;
using CommunityToolkit.Mvvm.Input;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Features.Host;

// - The host's settings: port and test mode.
public partial class HostViewModel
{
    /// <summary>Saves the new port, and the service rebinds its listener. Checks again that the
    /// port is free.</summary>
    internal async Task ApplyPortAsync(int port, CancellationToken cancellationToken)
    {
        if (!CanChangePort || !LocalControlProtocol.IsSettablePort(port) || port == CurrentPort) return;
        int? owner;
        try { owner = await Task.Run(() => HostNetwork.FindListener(port), cancellationToken); }
        catch (System.ComponentModel.Win32Exception)
        {
            ShowFeedback(HostMessages.PortCheckFailed, MessageTone.Error);
            return;
        }
        if (owner is { } listener)
        {
            ShowFeedback(HostMessages.PortInUse(port, HostNetwork.ProcessName(listener)) + ".", MessageTone.Error);
            return;
        }
        var confirmedHost = await BeginInteractionAsync(cancellationToken);
        if (confirmedHost is null)
        {
            ShowFeedback(HostMessages.PortNotChanged, MessageTone.Error);
            return;
        }
        try
        {
            var reply = await client.SetPortAsync(confirmedHost.Value, port, cancellationToken);
            if (reply is not LocalControlPortReply saved || saved.Port != port)
            {
                ShowFeedback(HostMessages.PortNotChanged, MessageTone.Error);
                return;
            }
        }
        catch (HostControlException) when (!cancellationToken.IsCancellationRequested)
        {
            ShowFeedback(HostMessages.PortNotChanged, MessageTone.Error);
            return;
        }
        finally { EndInteraction(); }

        await RefreshQuietlyAsync(cancellationToken);
        ShowFeedback(HostMessages.PortChanged(port), MessageTone.Success);
    }

    [RelayCommand(AllowConcurrentExecutions = true)]
    private Task ChangePortAsync(CancellationToken cancellationToken) =>
        WithDialogAsync(async dialogs =>
        {
            if (CurrentPort is not { } current) return;
            if (await dialogs.AskPortAsync(current) is { } chosen) await ApplyPortAsync(chosen, cancellationToken);
        });

    /// <summary>After a rejected or canceled change, the switch shows the service's confirmed value
    /// again.</summary>
    [RelayCommand]
    private async Task ChangeTestModeAsync(bool enabled, CancellationToken cancellationToken)
    {
        try { await SetTestModeAsync(enabled, cancellationToken); }
        catch (OperationCanceledException) { }
        finally { OnPropertyChanged(nameof(TestModeOn)); }
    }

    internal async Task SetTestModeAsync(bool enabled, CancellationToken cancellationToken)
    {
        if (!CanManage || enabled == TestModeOn) return;
        var confirmedHost = await BeginInteractionAsync(cancellationToken);
        if (confirmedHost is null)
        {
            ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error);
            return;
        }
        try
        {
            var reply = await client.SetTestModeAsync(confirmedHost.Value, enabled, cancellationToken);
            if (reply is LocalControlTestModeReply { TestMode: var saved } && saved == enabled)
                TestModeOn = enabled;
            else ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error);
        }
        catch (HostControlException) { ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error); }
        finally { EndInteraction(); }
        await RefreshQuietlyAsync(cancellationToken);
    }
}
