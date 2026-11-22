using CommunityToolkit.WinUI.Controls;
using Microsoft.UI.Xaml.Automation.Peers;

namespace MyRigConfig.Features.Host;

/// <summary>A clickable settings card that UI Automation can invoke. The toolkit's card reports the
/// Invoke pattern without implementing it, so Narrator and Voice Access can't activate
/// it.</summary>
public sealed partial class InvokableSettingsCard : SettingsCard
{
    protected override AutomationPeer OnCreateAutomationPeer() => new InvokableSettingsCardAutomationPeer(this);
}
