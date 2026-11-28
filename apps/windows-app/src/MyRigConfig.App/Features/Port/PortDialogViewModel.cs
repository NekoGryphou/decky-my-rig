using CommunityToolkit.Mvvm.ComponentModel;
using MyRigConfig.Core.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Features.Port;

/// <summary>Edits the service port: in range, different from the current one, and free.</summary>
public partial class PortDialogViewModel : ObservableObject
{
    private readonly int currentPort;
    private readonly IPortProbe probe;
    private Task validation = Task.CompletedTask;
    private double? pending;
    private bool validating;
    private bool closed;

    public PortDialogViewModel(int currentPort) : this(currentPort, new HostPortProbeService()) { }

    internal PortDialogViewModel(int currentPort, IPortProbe probe)
    {
        this.probe = probe;
        this.currentPort = currentPort;
        Error = "";
        PortText = currentPort.ToString(System.Globalization.CultureInfo.InvariantCulture);
    }

    /// <summary>NaN while the box is empty.</summary>
    [ObservableProperty] public partial double PortInput { get; set; }

    /// <summary>The text in the box. Each change is checked at once, so Save follows the typing.</summary>
    [ObservableProperty] public partial string PortText { get; set; } = "";

    partial void OnPortTextChanged(string value)
    {
        PortInput = double.TryParse(value, System.Globalization.NumberStyles.None,
            System.Globalization.CultureInfo.InvariantCulture, out var parsed) ? parsed : double.NaN;
    }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    public partial string Error { get; set; }

    [ObservableProperty] public partial bool CanSave { get; set; }

    public bool HasError => Error.Length > 0;

    /// <summary>The port to save. Only valid when <see cref="CanSave"/> is true.</summary>
    public int ChosenPort { get; private set; }

    internal async Task CloseAsync()
    {
        closed = true;
        pending = null;
        CanSave = false;
        await validation;
    }

    partial void OnPortInputChanged(double value)
    {
        if (closed) return;
        pending = value;
        CanSave = false;
        Error = "";
        if (validating) return;
        validating = true;
        validation = ValidatePendingAsync();
    }

    private async Task ValidatePendingAsync()
    {
        try
        {
            while (!closed && pending is { } value)
            {
                pending = null;
                PortCheck? check;
                try { check = await Task.Run(() => PortCheck.Of(value, currentPort, probe)); }
                // ! Whatever the lookup throws, the dialog shows "couldn't be checked" and stays usable: a stored
                // ! exception would resurface from CloseAsync, inside the page's finally.
                catch (Exception) { check = null; }
                if (closed) return;
                if (pending.HasValue) continue;
                if (check is null)
                {
                    ChosenPort = 0;
                    Error = PortMessages.CheckFailed;
                }
                else Apply(check);
            }
        }
        finally { validating = false; }
    }

    private void Apply(PortCheck check)
    {
        ChosenPort = check.Port;
        CanSave = check.Result == PortCheckResult.Available;
        Error = check.Result switch
        {
            PortCheckResult.OutOfRange => PortMessages.OutOfRange,
            PortCheckResult.InUse => SharedMessages.PortInUseSentence(check.Port, check.Owner),
            _ => "",
        };
    }
}
