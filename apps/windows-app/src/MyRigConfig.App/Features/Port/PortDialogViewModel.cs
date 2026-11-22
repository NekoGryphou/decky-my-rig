using CommunityToolkit.Mvvm.ComponentModel;
using MyRigConfig.Core.Host;

namespace MyRigConfig.Features.Port;

/// <summary>Edits the service port: in range, different from the current one, and free.</summary>
public partial class PortDialogViewModel : ObservableObject
{
    private readonly int currentPort;
    private readonly Func<int, int?> findListener;
    private readonly Func<int, string?> processName;
    private Task validation = Task.CompletedTask;
    private double? pending;
    private bool validating;
    private bool closed;

    public PortDialogViewModel(int currentPort) : this(currentPort, HostNetwork.FindListener, HostNetwork.ProcessName) { }

    internal PortDialogViewModel(int currentPort, Func<int, int?> findListener, Func<int, string?> processName)
    {
        this.findListener = findListener;
        this.processName = processName;
        this.currentPort = currentPort;
        Error = "";
        PortInput = currentPort;
    }

    /// <summary>NaN while the box is empty.</summary>
    [ObservableProperty] public partial double PortInput { get; set; }

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
                try { check = await Task.Run(() => PortCheck.Of(value, currentPort, findListener, processName)); }
                catch (System.ComponentModel.Win32Exception) { check = null; }
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
            PortCheckResult.InUse => PortMessages.InUse(check.Port, check.Owner),
            _ => "",
        };
    }
}
