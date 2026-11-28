using MyRigConfig.Shared;
using MyRigConfig.Core.Shared;
using CommunityToolkit.Mvvm.ComponentModel;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Management;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Features.Pairing;

/// <summary>One pairing attempt: name the device, show a short-lived code, and watch the host's
/// paired list until the device shows up. The code is cleared as soon as it expires, pairs, or the
/// dialog closes.</summary>
public partial class PairingViewModel : ObservableObject
{
    private readonly HostViewModel host;
    private readonly SemaphoreSlim operation = new(1, 1);
    private readonly CancellationTokenSource lifetime = new();
    private Task? closing;
    private bool closed;
    private Guid? approvalHost;

    private Guid? confirmedHost;
    private PairingCodeLeaseService? lease;
    private IReadOnlySet<Guid> devicesBefore = new HashSet<Guid>();
    private string requestedName = "";
    private int ticks;

    public event EventHandler? Paired;

    public PairingViewModel(HostViewModel host)
    {
        this.host = host;
        Step = PairingStep.Preparing;
        DeviceName = PairingMessages.DefaultDeviceName;
        ErrorMessage = "";
        CodeText = "";
        RemainingPercent = 100;
        RemainingText = "";
    }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Title), nameof(PrimaryText), nameof(CloseText), nameof(CanCreate),
        nameof(IsNameStep), nameof(IsCodeStep))]
    public partial PairingStep Step { get; set; }

    [ObservableProperty] public partial string DeviceName { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    public partial string ErrorMessage { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(CodeAccessibleName))]
    public partial string CodeText { get; set; }

    public string CodeAccessibleName => PairingMessages.CodeName(CodeText);
    [ObservableProperty] public partial double RemainingPercent { get; set; }
    [ObservableProperty] public partial string RemainingText { get; set; } = "";

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(CanCreate))]
    public partial bool IsWorking { get; set; }

    public bool IsNameStep => Step is PairingStep.Name or PairingStep.Preparing;
    public bool IsCodeStep => Step == PairingStep.Code;
    public bool HasError => ErrorMessage.Length > 0;
    public bool CanCreate => !closed && !IsWorking && Step is PairingStep.Name or PairingStep.Expired or PairingStep.Failed;

    public string Title => Step switch
    {
        PairingStep.Code => PairingMessages.TitleCode,
        PairingStep.Expired => PairingMessages.TitleExpired,
        PairingStep.Failed => PairingMessages.TitleFailed,
        _ => PairingMessages.TitleName,
    };

    public string PrimaryText => Step switch
    {
        PairingStep.Code => "",
        PairingStep.Expired or PairingStep.Failed => PairingMessages.NewCode,
        _ => PairingMessages.CreateCode,
    };

    public string CloseText => Step is PairingStep.Expired or PairingStep.Failed ? PairingMessages.Close : SharedMessages.Cancel;

    internal async Task StartAsync(CancellationToken cancellationToken)
    {
        if (closed) return;
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, lifetime.Token);
        await operation.WaitAsync(linked.Token);
        IsWorking = true;
        try
        {
            if (confirmedHost is null) confirmedHost = await host.BeginInteractionAsync(linked.Token);
            if (confirmedHost is null) Fail(PairingMessages.Unreachable);
            else Step = PairingStep.Name;
        }
        finally { IsWorking = false; operation.Release(); }
    }

    internal async Task CreateCodeAsync(CancellationToken cancellationToken)
    {
        if (!CanCreate) return;
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, lifetime.Token);
        await operation.WaitAsync(linked.Token);
        try
        {
            if (!CanCreate) return;
            IsWorking = true;
            ErrorMessage = "";
            ClearCode();
            requestedName = DeviceName.Trim();
            if (!LocalControlProtocolService.IsValidName(requestedName))
            {
                ErrorMessage = PairingMessages.InvalidName;
                Step = PairingStep.Name;
                return;
            }
            if (confirmedHost is null) confirmedHost = await host.BeginInteractionAsync(linked.Token);
            if (confirmedHost is not { } hostId)
            {
                Fail(PairingMessages.Unreachable);
                return;
            }
            devicesBefore = host.CurrentDeviceIds;
            var (newLease, failure, _) = await OpenCodeAsync(hostId, linked.Token);
            if (linked.IsCancellationRequested)
            {
                newLease?.Dispose();
                linked.Token.ThrowIfCancellationRequested();
            }
            if (newLease?.Code is not { } code)
            {
                newLease?.Dispose();
                await CancelApprovalAsync();
                ReleaseConfirmation();
                Fail(PairingMessages.Failure(failure ?? PairingFailure.CodeFailed));
                return;
            }
            lease = newLease;
            ticks = 0;
            CodeText = PairingMessages.FormatCode(code);
            Step = PairingStep.Code;
            if (!UpdateCountdown()) await CancelApprovalAsync();
        }
        finally { IsWorking = false; operation.Release(); }
    }

    private async Task<PairingOpenResult> OpenCodeAsync(Guid hostId, CancellationToken cancellationToken)
    {
        // ! Assume an approval exists until the service says otherwise, so canceling after the request went out still
        // ! closes it.
        var previousApproval = approvalHost;
        approvalHost = hostId;
        var result = await host.OpenPairingAsync(hostId, requestedName, cancellationToken);
        if (!result.ApprovalMayExist) approvalHost = previousApproval;
        return result;
    }

    /// <summary>Called once a second while the code is shown.</summary>
    internal async Task TickAsync(CancellationToken cancellationToken)
    {
        if (closed || Step != PairingStep.Code || lease is null) return;
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, lifetime.Token);
        if (!await operation.WaitAsync(0, linked.Token)) return;
        try
        {
            if (!UpdateCountdown())
            {
                IsWorking = true;
                await CancelApprovalAsync();
                return;
            }
            if (++ticks % PairingConstants.PollEveryTicks != 0) return;
            if (await host.HasNewDeviceAsync(devicesBefore, requestedName, linked.Token))
            {
                linked.Token.ThrowIfCancellationRequested();
                ClearCode();
                approvalHost = null; // - Successful pairing consumed this approval.
                host.ApprovalConsumed();
                host.ShowFeedback(PairingMessages.Paired(requestedName), MessageTone.Success);
                Paired?.Invoke(this, EventArgs.Empty);
            }
        }
        finally { IsWorking = false; operation.Release(); }
    }

    internal Task CloseAsync() => closing ??= CloseCoreAsync();

    private async Task CloseCoreAsync()
    {
        closed = true;
        OnPropertyChanged(nameof(CanCreate));
        ClearCode();
        lifetime.Cancel();
        // ! Let the client exchange finish before sending close with its own deadline. The host invalidates older
        // ! approval callbacks, queued work included.
        await operation.WaitAsync();
        try { await CancelApprovalAsync(); }
        finally
        {
            ReleaseConfirmation();
            operation.Release();
            lifetime.Dispose();
        }
    }

    private async Task CancelApprovalAsync()
    {
        if (approvalHost is not { } hostId) return;
        approvalHost = null;
        if (!await host.ClosePairingAsync(hostId))
            host.ShowFeedback(PairingMessages.CancelFailed, MessageTone.Warning);
    }

    private void ReleaseConfirmation()
    {
        if (confirmedHost is not null) host.EndInteraction();
        confirmedHost = null;
    }

    private bool UpdateCountdown()
    {
        var remaining = lease?.Remaining ?? TimeSpan.Zero;
        if (lease?.Code is null || remaining <= TimeSpan.Zero)
        {
            ClearCode();
            Step = PairingStep.Expired;
            return false;
        }
        RemainingPercent = remaining / PairingProtocolConstants.Lifetime * 100;
        RemainingText = PairingMessages.ExpiresIn(remaining);
        return true;
    }

    internal void FailUnexpectedly() => Fail(PairingMessages.CodeFailed);

    private void Fail(string message)
    {
        ClearCode();
        ErrorMessage = message;
        Step = PairingStep.Failed;
    }

    private void ClearCode()
    {
        lease?.Dispose();
        lease = null;
        CodeText = "";
    }
}
