using MyRigConfig.Shared;
using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Management;

namespace MyRigConfig.Features.Host;

/// <summary>Service status, paired devices and settings. Every change goes through <see
/// cref="HostManagementService"/>, which checks the host again against the one the user
/// confirmed.</summary>
public partial class HostViewModel : ObservableObject
{
    private readonly HostManagementService management = new();
    private readonly HostControlClientService client;
    private readonly Func<CancellationToken, Task<HostObservation>> diagnostics;
    private readonly TimeProvider clock;
    private readonly Func<CancellationToken, Task> startService;
    private readonly Func<CancellationToken, Task> restartService;
    private readonly IPortProbe probe;
    private int interactions;
    // ! The host whose pairing code may still be valid. Retiring the page closes it.
    private Guid? openApproval;

    public HostViewModel() : this(new HostControlClientService(), HostDiagnosticsService.ReadAsync, TimeProvider.System,
        HostServiceControlService.StartAsync, HostServiceControlService.RestartAsync, new HostPortProbeService()) { }

    internal HostViewModel(HostControlClientService client,
        Func<CancellationToken, Task<HostObservation>> diagnostics, TimeProvider clock,
        Func<CancellationToken, Task> startService, Func<CancellationToken, Task> restartService,
        IPortProbe probe)
    {
        this.probe = probe;
        this.startService = startService;
        this.restartService = restartService;
        this.client = client;
        this.diagnostics = diagnostics;
        this.clock = clock;
        ServiceState = ServiceState.Checking;
        FeedbackMessage = "";
        PairedDevices.CollectionChanged += (_, _) => OnPropertyChanged(nameof(ShowNoDevices));
    }

    public ObservableCollection<ManagedDeviceSummary> PairedDevices { get; } = [];

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(ServiceStatusText), nameof(ShowStartButton), nameof(ShowRestartButton),
        nameof(IsServiceRunning), nameof(IsServiceDown), nameof(IsServicePending), nameof(NoDevicesText),
        nameof(ShowNoDevices))]
    [NotifyCanExecuteChangedFor(nameof(StartServiceCommand), nameof(RestartServiceCommand))]
    public partial ServiceState ServiceState { get; set; }

    /// <summary>Service port, or null when local control can't be read.</summary>
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(CanChangePort), nameof(Port), nameof(PortAccessibleName))]
    public partial int? CurrentPort { get; set; }

    public string Port => CurrentPort?.ToString(System.Globalization.CultureInfo.InvariantCulture) ?? "";

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasPortConflict), nameof(PortConflictText), nameof(PortAccessibleName))]
    public partial PortConflict? PortConflict { get; set; }

    public string PortAccessibleName => HasPortConflict ? PortConflictText : HostMessages.PortName(Port);
    public bool HasPortConflict => PortConflict is not null;
    public string PortConflictText => PortConflict is { } conflict ? HostMessages.PortInUse(conflict.Port, conflict.ProcessName) : "";

    [ObservableProperty] public partial bool ListenerUnavailable { get; set; }

    [ObservableProperty]
    [NotifyCanExecuteChangedFor(nameof(RefreshCommand), nameof(StartServiceCommand), nameof(RestartServiceCommand))]
    public partial bool IsBusy { get; set; }

    [ObservableProperty]
    [NotifyCanExecuteChangedFor(nameof(RefreshCommand), nameof(StartServiceCommand), nameof(RestartServiceCommand))]
    public partial bool IsInteracting { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(CanChangePort))]
    public partial bool CanManage { get; set; }

    [ObservableProperty] public partial bool ControlUnavailable { get; set; }

    [ObservableProperty] public partial bool ControlUnauthorized { get; set; }

    [ObservableProperty] public partial bool TestModeOn { get; set; }

    [ObservableProperty] public partial string FeedbackMessage { get; set; }
    [ObservableProperty] public partial MessageTone FeedbackSeverity { get; set; }
    [ObservableProperty] public partial bool IsFeedbackOpen { get; set; }

    public bool CanChangePort => CanManage && CurrentPort is not null;

    // ? If the list can't be read, the page notice already says why, so no second bar.
    public bool ShowNoDevices => PairedDevices.Count == 0
        && (management.Available || ServiceState == ServiceState.Checking);
    public string NoDevicesText => management.Available ? HostMessages.NoPairedDevice : HostMessages.StatusChecking;

    public string ServiceStatusText => ServiceState switch
    {
        ServiceState.Running => HostMessages.StatusRunning,
        ServiceState.Stopped => HostMessages.StatusStopped,
        ServiceState.Starting => HostMessages.StatusStarting,
        ServiceState.Stopping => HostMessages.StatusStopping,
        ServiceState.NotInstalled => HostMessages.StatusNotInstalled,
        ServiceState.Unavailable => HostMessages.StatusUnavailable,
        _ => HostMessages.StatusChecking,
    };

    /// <summary>The one persistent notice; see <see cref="HostHelpers.SelectNotice"/>.</summary>
    [ObservableProperty]
    // ? Set the text first: the bar reopens with the new text, and Narrator reads it.
    [NotifyPropertyChangedFor(nameof(NoticeTitle), nameof(NoticeMessage), nameof(NoticeSeverity), nameof(IsNoticeOpen))]
    public partial HostNotice? Notice { get; private set; }

    partial void OnServiceStateChanged(ServiceState value) => UpdateNotice();
    partial void OnPortConflictChanged(PortConflict? value) => UpdateNotice();
    partial void OnListenerUnavailableChanged(bool value) => UpdateNotice();
    partial void OnControlUnavailableChanged(bool value) => UpdateNotice();
    partial void OnControlUnauthorizedChanged(bool value) => UpdateNotice();
    partial void OnTestModeOnChanged(bool value) => UpdateNotice();

    private void UpdateNotice() => Notice = HostHelpers.SelectNotice(ServiceState, PortConflict,
        ListenerUnavailable, ControlUnauthorized, ControlUnavailable, TestModeOn);

    // * A notice replaced by another is announced like a new one (see ShowFeedback).
    partial void OnNoticeChanged(HostNotice? oldValue, HostNotice? newValue)
    {
        if (oldValue is null || newValue is null) return;
        reopeningNotice = true;
        OnPropertyChanged(nameof(IsNoticeOpen));
        reopeningNotice = false;
    }

    private bool reopeningNotice;
    public bool IsNoticeOpen => Notice is not null && !reopeningNotice;
    public string NoticeTitle => Notice?.Title ?? "";
    public string NoticeMessage => Notice?.Message ?? "";
    public MessageTone NoticeSeverity => Notice?.Severity ?? MessageTone.Informational;
    public bool ShowStartButton => ServiceState == ServiceState.Stopped;
    public bool ShowRestartButton => ServiceState == ServiceState.Running;
    public bool IsServiceRunning => ServiceState == ServiceState.Running;
    public bool IsServiceDown => ServiceState is ServiceState.Stopped or ServiceState.NotInstalled or ServiceState.Unavailable;
    public bool IsServicePending => ServiceState is ServiceState.Checking or ServiceState.Starting or ServiceState.Stopping;

    internal IReadOnlySet<Guid> CurrentDeviceIds => PairedDevices.Select(device => device.DeviceId).ToHashSet();

    // ? Narrator announces an InfoBar when it opens, not when its text changes, so an open bar is closed and reopened
    // ? for each new message.
    internal void ShowFeedback(string message, MessageTone severity)
    {
        IsFeedbackOpen = false;
        FeedbackMessage = message;
        FeedbackSeverity = severity;
        IsFeedbackOpen = true;
    }

    private bool CanRefresh() => !IsBusy && !IsInteracting;
    private bool CanStartService() => CanRefresh() && ServiceState == ServiceState.Stopped;
    private bool CanRestartService() => CanRefresh() && ServiceState == ServiceState.Running;

    [RelayCommand(CanExecute = nameof(CanRefresh))]
    private Task RefreshAsync(CancellationToken cancellationToken) => RefreshCoreAsync(quiet: false, cancellationToken);

    /// <summary>Periodic refresh that doesn't disable controls or flash progress.</summary>
    internal Task RefreshQuietlyAsync(CancellationToken cancellationToken) =>
        CanRefresh() ? RefreshCoreAsync(quiet: true, cancellationToken) : Task.CompletedTask;

    private async Task RefreshCoreAsync(bool quiet, CancellationToken cancellationToken)
    {
        if (quiet)
        {
            if (!management.TryBegin()) return;
        }
        else
        {
            try { await management.BeginAsync(cancellationToken); }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
        }
        if (!quiet)
        {
            IsBusy = true;
            UpdateCanManage();
        }
        try
        {
            var observation = await diagnostics(cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            ServiceState = observation.ServiceState;
            ApplyStatus(observation.Status);
            PortConflict = observation.PortConflict;
            if (ServiceState == ServiceState.Running) await RefreshManagementAsync(cancellationToken);
            else ReportUnavailable(controlFailure: false);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        // ! Only failures of the host itself mean "unavailable"; a bug must not look like one.
        catch (Exception exception) when (!cancellationToken.IsCancellationRequested
            && exception is HostControlException or IOException or System.ComponentModel.Win32Exception)
        {
            ServiceState = ServiceState.Unavailable;
            ReportUnavailable(controlFailure: false);
        }
        finally
        {
            management.End();
            if (!quiet) IsBusy = false;
            UpdateCanManage();
        }
    }

    /// <summary>Called when the page is torn down: stops everything and closes a pairing code that
    /// is still open, because nothing is left to watch it.</summary>
    internal async Task RetireAsync()
    {
        CancelAll();
        if (openApproval is { } hostId) await ClosePairingAsync(hostId);
    }

    private void CancelAll()
    {
        RefreshCommand.Cancel();
        StartServiceCommand.Cancel();
        RestartServiceCommand.Cancel();
        PairCommand.Cancel();
        RemoveCommand.Cancel();
        RenameCommand.Cancel();
        ChangePortCommand.Cancel();
        ChangeTestModeCommand.Cancel();
    }

    [RelayCommand(CanExecute = nameof(CanStartService))]
    private Task StartServiceAsync(CancellationToken cancellationToken) =>
        ControlServiceAsync(startService, ServiceState.Starting, HostMessages.StartFailed, cancellationToken);

    [RelayCommand(CanExecute = nameof(CanRestartService))]
    private Task RestartServiceAsync(CancellationToken cancellationToken) =>
        ControlServiceAsync(restartService, ServiceState.Stopping, HostMessages.RestartFailed, cancellationToken);

    private async Task ControlServiceAsync(Func<CancellationToken, Task> action, ServiceState pending,
        string failure, CancellationToken cancellationToken)
    {
        try { await management.BeginAsync(cancellationToken); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
        IsBusy = true;
        ServiceState = pending;
        UpdateCanManage();
        try { await action(cancellationToken); }
        catch (HostServiceException)
        {
            if (!cancellationToken.IsCancellationRequested) ShowFeedback(failure, MessageTone.Error);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        finally
        {
            management.End();
            IsBusy = false;
        }
        // ? SCM work can finish after the command was cancelled, so the final state is still published. Only a retired
        // ? page, which nobody looks at any more, skips it.
        await RefreshAsync(PageLifetime);
    }

    /// <summary>Starts a user interaction and returns the host it was confirmed against.</summary>
    internal async Task<Guid?> BeginInteractionAsync(CancellationToken cancellationToken)
    {
        interactions++;
        IsInteracting = true;
        UpdateCanManage();
        var acquired = false;
        var started = false;
        try
        {
            await management.BeginAsync(cancellationToken);
            acquired = true;
            if (await RefreshManagementAsync(cancellationToken))
            {
                started = true;
                return management.RequireHost();
            }
            return null;
        }
        finally
        {
            if (!started)
            {
                if (acquired) management.End();
                CompleteInteraction();
            }
        }
    }

    internal void EndInteraction()
    {
        management.End();
        CompleteInteraction();
    }

    private void CompleteInteraction()
    {
        IsInteracting = --interactions > 0;
        UpdateCanManage();
    }

    /// <summary>Opens a pairing code for <paramref name="name"/> on the confirmed host.</summary>
    internal async Task<PairingOpenResult> OpenPairingAsync(Guid confirmedHost, string name,
        CancellationToken cancellationToken)
    {
        if (!await RefreshManagementAsync(cancellationToken)) return new(null, PairingFailure.Unreachable, false);
        if (!management.MatchesHost(confirmedHost)) return new(null, PairingFailure.HostChanged, false);
        if (!LocalControlProtocolService.IsValidName(name)) return new(null, PairingFailure.InvalidName, false);
        // ! The code's lifetime starts counting before the request is sent.
        var started = clock.GetTimestamp();
        var sent = false;
        // ! The code may exist from the moment the request goes out, so it is recorded then.
        var previousApproval = openApproval;
        try
        {
            var reply = await client.OpenPairingAsync(confirmedHost, name,
                () => { sent = true; openApproval = confirmedHost; }, cancellationToken);
            PairingOpenResult result = reply switch
            {
                LocalControlPairingReply pairing => new(new PairingCodeLeaseService(pairing.Code, started, clock), null, true),
                LocalControlErrorReply => new(null, PairingFailure.CodeFailed, false),
                _ => new(null, PairingFailure.CodeFailed, sent),
            };
            if (!result.ApprovalMayExist) openApproval = previousApproval;
            return result;
        }
        catch (HostControlException exception)
        {
            ReportUnavailable(controlFailure: true);
            if (!exception.OutcomeUncertain)
            {
                openApproval = previousApproval;
                return new(null, PairingFailure.Unreachable, false);
            }
            return new(null, PairingFailure.CodeFailed, true);
        }
    }

    /// <summary>A device paired with the open code, so the host no longer holds it.</summary>
    internal void ApprovalConsumed() => openApproval = null;

    internal async Task<bool> ClosePairingAsync(Guid hostId)
    {
        try
        {
            var closed = await client.ClosePairingAsync(hostId, CancellationToken.None) is LocalControlClosePairingReply;
            // ! A failed close stays recorded, so retiring the page tries once more.
            if (closed && openApproval == hostId) openApproval = null;
            return closed;
        }
        catch (HostControlException) { return false; }
    }

    /// <summary>True once a device that was not paired before has paired under <paramref name="name"/>.</summary>
    internal async Task<bool> HasNewDeviceAsync(IReadOnlySet<Guid> before, string name, CancellationToken cancellationToken) =>
        await RefreshManagementAsync(cancellationToken)
        && PairedDevices.Any(device => !before.Contains(device.DeviceId) && string.Equals(device.Name, name, StringComparison.Ordinal));

    /// <summary>Runs <paramref name="action"/> against the host the user confirmed and releases the
    /// interaction afterwards. Returns false when no host could be confirmed; then
    /// <paramref name="unreachable"/>, if given, is shown.</summary>
    private async Task<bool> InteractAsync(Func<Guid, Task> action, string? unreachable, CancellationToken cancellationToken)
    {
        var confirmedHost = await BeginInteractionAsync(cancellationToken);
        if (confirmedHost is null)
        {
            if (unreachable is not null) ShowFeedback(unreachable, MessageTone.Error);
            return false;
        }
        try { await action(confirmedHost.Value); }
        finally { EndInteraction(); }
        return true;
    }

    internal Task RemoveDeviceAsync(Guid deviceId, Func<string, Task<bool>> confirm, CancellationToken cancellationToken) =>
        InteractAsync(async confirmedHost =>
        {
            var name = DeviceName(deviceId);
            if (name is null || !await confirm(name)) return;
            await ChangeDeviceAsync(confirmedHost, deviceId, name, ct => RemovedAsync(confirmedHost, deviceId, ct),
                HostMessages.Removed(name), HostMessages.RemovalUncertain, cancellationToken);
        }, unreachable: null, cancellationToken);

    /// <param name="askName">Returns the new name, or null when the user cancels.</param>
    internal Task RenameDeviceAsync(Guid deviceId, Func<string, Task<string?>> askName, CancellationToken cancellationToken) =>
        InteractAsync(async confirmedHost =>
        {
            var name = DeviceName(deviceId);
            if (name is null) return;
            var newName = await askName(name);
            if (newName is null || string.Equals(newName, name, StringComparison.Ordinal)
                || !LocalControlProtocolService.IsValidName(newName)) return;
            await ChangeDeviceAsync(confirmedHost, deviceId, name, ct => RenamedAsync(confirmedHost, deviceId, newName, ct),
                HostMessages.Renamed(newName), HostMessages.RenameUncertain, cancellationToken);
        }, unreachable: null, cancellationToken);

    private string? DeviceName(Guid deviceId) => PairedDevices.FirstOrDefault(device => device.DeviceId == deviceId)?.Name;

    /// <summary>After the user answers, reads the list again. The change only goes through if the
    /// device is still listed under the same name on the same host.</summary>
    private async Task<bool> StillListedAsync(Guid host, Guid deviceId, string name, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (!await RefreshManagementAsync(cancellationToken)) return false;
        if (management.MatchesDevice(host, deviceId, name)) return true;
        ShowFeedback(HostMessages.ListChanged, MessageTone.Warning);
        return false;
    }

    /// <summary>Set by the page that shows this view model's dialogs.</summary>
    internal IHostDialogs? Dialogs { get; set; }

    /// <summary>While a dialog is open, background refreshes wait and other dialog commands do
    /// nothing. The commands allow concurrent runs on purpose: otherwise the toolkit disables the
    /// control that opened the dialog, and it loses keyboard focus.</summary>
    internal bool IsDialogOpen { get; private set; }

    /// <summary>Cancelled when the page is torn down. Work that outlives a cancelled command still
    /// stops with the page.</summary>
    internal CancellationToken PageLifetime { get; set; }

    /// <summary>Where unexpected exceptions go. Tests replace it.</summary>
    internal Action<Exception> LogError { get; set; } = ErrorLogService.Write;

    // ! These commands run from async void event handlers, so an exception that escapes here would end the app. It is
    // ! logged and shown as the command's own failure message instead.
    private async Task WithDialogAsync(Func<IHostDialogs, Task> action, string failure)
    {
        if (IsDialogOpen || Dialogs is not { } dialogs) return;
        IsDialogOpen = true;
        try { await action(dialogs); }
        catch (OperationCanceledException) { }
        catch (Exception exception)
        {
            LogError(exception);
            ShowFeedback(failure, MessageTone.Error);
        }
        finally { IsDialogOpen = false; }
    }

    [RelayCommand(AllowConcurrentExecutions = true)]
    private Task PairAsync(CancellationToken cancellationToken) =>
        WithDialogAsync(dialogs => dialogs.ShowPairingAsync(cancellationToken), HostMessages.PairFailed);

    [RelayCommand(AllowConcurrentExecutions = true)]
    private Task RemoveAsync(Guid deviceId, CancellationToken cancellationToken) =>
        WithDialogAsync(dialogs => RemoveDeviceAsync(deviceId, dialogs.ConfirmRemovalAsync, cancellationToken),
            HostMessages.RemovalUncertain);

    [RelayCommand(AllowConcurrentExecutions = true)]
    private Task RenameAsync(Guid deviceId, CancellationToken cancellationToken) =>
        WithDialogAsync(dialogs => RenameDeviceAsync(deviceId, dialogs.AskNameAsync, cancellationToken),
            HostMessages.RenameUncertain);

    private void ApplyStatus(HostStatus? status)
    {
        CurrentPort = status?.Port;
        TestModeOn = status is { TestMode: true };
        ListenerUnavailable = status is { ListenerState: LocalControlConstants.ListenerUnavailableState };
    }

    private async Task<bool> RefreshManagementAsync(CancellationToken cancellationToken)
    {
        try
        {
            var reply = await client.ListAsync(cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            if (reply is not LocalControlListReply list)
            {
                ReportUnavailable(controlFailure: true,
                    unauthorized: reply is LocalControlErrorReply { Error: LocalControlConstants.UnauthorizedError });
                return false;
            }
            management.Publish(list);
            // ? Only rebuild when the set changed, so focus and scroll position survive periodic refreshes.
            if (!PairedDevices.Select(device => (device.DeviceId, device.Name))
                    .SequenceEqual(management.Devices.Select(device => (device.DeviceId, device.Name))))
            {
                PairedDevices.Clear();
                foreach (var device in management.Devices) PairedDevices.Add(new ManagedDeviceSummary(device, RenameCommand, RemoveCommand));
            }
            ControlUnavailable = false;
            ControlUnauthorized = false;
            UpdateCanManage();
            return true;
        }
        catch (HostControlException exception)
        {
            ReportUnavailable(controlFailure: true, exception.Unauthorized);
            return false;
        }
    }

    private void ReportUnavailable(bool controlFailure, bool unauthorized = false)
    {
        management.Unavailable();
        if (PairedDevices.Count > 0) PairedDevices.Clear();
        ControlUnavailable = controlFailure;
        ControlUnauthorized = unauthorized;
        UpdateCanManage();
    }

    private void UpdateCanManage()
    {
        CanManage = management.Available && !IsBusy && !IsInteracting;
        foreach (var device in PairedDevices) device.CanEdit = CanManage;
        OnPropertyChanged(nameof(NoDevicesText));
        OnPropertyChanged(nameof(ShowNoDevices));
    }
}
