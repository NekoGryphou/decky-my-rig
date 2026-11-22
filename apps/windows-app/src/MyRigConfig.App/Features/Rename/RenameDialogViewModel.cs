using MyRigConfig.Shared;
using CommunityToolkit.Mvvm.ComponentModel;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Features.Rename;

/// <summary>Edits a device name: valid by the service's rules and different from the current
/// one.</summary>
public partial class RenameDialogViewModel : ObservableObject
{
    private readonly string currentName;

    public RenameDialogViewModel(string currentName)
    {
        this.currentName = currentName;
        Error = "";
        ChosenName = currentName;
        Name = currentName;
    }

    [ObservableProperty] public partial string Name { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    public partial string Error { get; set; }

    [ObservableProperty] public partial bool CanSave { get; set; }

    public bool HasError => Error.Length > 0;

    /// <summary>The trimmed name to save. Only valid when <see cref="CanSave"/> is true.</summary>
    public string ChosenName { get; private set; }

    partial void OnNameChanged(string value)
    {
        ChosenName = value.Trim();
        var valid = LocalControlProtocol.IsValidName(ChosenName);
        Error = valid ? "" : SharedMessages.InvalidDeviceName;
        CanSave = valid && !string.Equals(ChosenName, currentName, StringComparison.Ordinal);
    }
}
