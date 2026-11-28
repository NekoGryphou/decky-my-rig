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
        if (!CanChangePort || CurrentPort is not { } current) return;
        PortCheck check;
        try { check = await Task.Run(() => PortCheck.Of(port, current, probe), cancellationToken); }
        catch (System.ComponentModel.Win32Exception)
        {
            ShowFeedback(HostMessages.PortCheckFailed, MessageTone.Error);
            return;
        }
        if (check.Result == PortCheckResult.InUse)
        {
            ShowFeedback(SharedMessages.PortInUseSentence(port, check.Owner), MessageTone.Error);
            return;
        }
        if (check.Result != PortCheckResult.Available) return;
        var saved = false;
        var confirmed = await InteractAsync(async host => saved = await SendPortAsync(host, port, cancellationToken),
            HostMessages.PortNotChanged, cancellationToken);
        if (!confirmed || !saved) return;
        await RefreshQuietlyAsync(cancellationToken);
        ShowFeedback(HostMessages.PortChanged(port), MessageTone.Success);
    }

    private async Task<bool> SendPortAsync(Guid host, int port, CancellationToken cancellationToken)
    {
        try
        {
            var reply = await client.SetPortAsync(host, port, cancellationToken);
            if (reply is LocalControlPortReply saved && saved.Port == port) return true;
        }
        catch (HostControlException) when (!cancellationToken.IsCancellationRequested) { }
        ShowFeedback(HostMessages.PortNotChanged, MessageTone.Error);
        return false;
    }

    [RelayCommand(AllowConcurrentExecutions = true)]
    private Task ChangePortAsync(CancellationToken cancellationToken) =>
        WithDialogAsync(async dialogs =>
        {
            if (CurrentPort is not { } current) return;
            if (await dialogs.AskPortAsync(current) is { } chosen) await ApplyPortAsync(chosen, cancellationToken);
        }, HostMessages.PortNotChanged);

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
        if (!await InteractAsync(host => SendTestModeAsync(host, enabled, cancellationToken),
                HostMessages.TestModeChangeFailed, cancellationToken)) return;
        await RefreshQuietlyAsync(cancellationToken);
    }

    private async Task SendTestModeAsync(Guid host, bool enabled, CancellationToken cancellationToken)
    {
        try
        {
            var reply = await client.SetTestModeAsync(host, enabled, cancellationToken);
            if (reply is LocalControlTestModeReply { TestMode: var saved } && saved == enabled) TestModeOn = enabled;
            else ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error);
        }
        catch (HostControlException) { ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error); }
    }

    /// <summary>Logs a test-mode failure that escaped the command's own handling and tells the user.</summary>
    internal void ReportTestModeFailed(Exception exception)
    {
        LogError(exception);
        ShowFeedback(HostMessages.TestModeChangeFailed, MessageTone.Error);
    }
}
