using CommunityToolkit.WinUI.Controls;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Automation.Provider;

namespace MyRigConfig.Features.Host;

/// <summary>Invoking runs the card's command, just like a click.</summary>
public sealed partial class InvokableSettingsCardAutomationPeer(InvokableSettingsCard owner)
    : SettingsCardAutomationPeer(owner), IInvokeProvider
{
    protected override object? GetPatternCore(PatternInterface patternInterface) =>
        patternInterface == PatternInterface.Invoke ? this : base.GetPatternCore(patternInterface);

    public void Invoke()
    {
        if (owner.IsEnabled && owner.Command is { } command && command.CanExecute(owner.CommandParameter))
            command.Execute(owner.CommandParameter);
    }
}
