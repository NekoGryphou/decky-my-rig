using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MyRigConfig.Features.Rename;

public sealed partial class RenameDialog : ContentDialog
{
    public RenameDialog(string currentName)
    {
        ViewModel = new RenameDialogViewModel(currentName);
        InitializeComponent();
        Opened += (_, _) =>
        {
            NameBox.Focus(FocusState.Programmatic);
            NameBox.SelectAll();
        };
    }

    public RenameDialogViewModel ViewModel { get; }
}
