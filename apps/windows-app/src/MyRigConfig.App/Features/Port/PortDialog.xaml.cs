using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MyRigConfig.Features.Port;

public sealed partial class PortDialog : ContentDialog
{
    public PortDialog(int currentPort)
    {
        ViewModel = new PortDialogViewModel(currentPort);
        InitializeComponent();
        Opened += (_, _) => PortBox.Focus(FocusState.Programmatic);
    }

    public PortDialogViewModel ViewModel { get; }
}
