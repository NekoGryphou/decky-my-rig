using MyRigConfig.Shared;
using Microsoft.UI.Xaml.Controls;

namespace MyRigConfig.Features.Host;

public static class HostPageHelpers
{
    public static InfoBarSeverity ToSeverity(MessageTone tone) => tone switch
    {
        MessageTone.Success => InfoBarSeverity.Success,
        MessageTone.Warning => InfoBarSeverity.Warning,
        MessageTone.Error => InfoBarSeverity.Error,
        _ => InfoBarSeverity.Informational,
    };
}
