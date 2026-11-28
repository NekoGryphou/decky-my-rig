using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml.Controls;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Features.Pairing;

public sealed partial class PairingDialog : ContentDialog
{
    private readonly CancellationToken pageLifetime;
    private readonly DispatcherQueueTimer timer;
    private bool closed;

    public PairingDialog(HostViewModel host, CancellationToken pageLifetime)
    {
        ViewModel = new PairingViewModel(host);
        this.pageLifetime = pageLifetime;
        InitializeComponent();

        timer = DispatcherQueue.CreateTimer();
        timer.Interval = PairingConstants.TickInterval;
        timer.Tick += Timer_Tick;
        ViewModel.Paired += (_, _) => Hide();
        Opened += Dialog_Opened;
        Closed += Dialog_Closed;
        PrimaryButtonClick += Dialog_PrimaryButtonClick;
    }

    public PairingViewModel ViewModel { get; }

    private async void Dialog_Opened(ContentDialog sender, ContentDialogOpenedEventArgs args)
    {
        try
        {
            await ViewModel.StartAsync(pageLifetime);
            if (!closed && ViewModel.IsNameStep) NameBox.Focus(Microsoft.UI.Xaml.FocusState.Programmatic);
        }
        catch (OperationCanceledException) { }
        catch (Exception) { ViewModel.FailUnexpectedly(); }
    }

    private async void Dialog_PrimaryButtonClick(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        // - The dialog stays open; each step decides what happens next.
        args.Cancel = true;
        var deferral = args.GetDeferral();
        try
        {
            await ViewModel.CreateCodeAsync(pageLifetime);
            if (!closed && ViewModel.IsCodeStep) timer.Start();
        }
        catch (OperationCanceledException) { }
        catch (Exception) { ViewModel.FailUnexpectedly(); }
        finally { deferral.Complete(); }
    }

    private async void Timer_Tick(DispatcherQueueTimer sender, object args)
    {
        if (closed) return;
        try
        {
            await ViewModel.TickAsync(pageLifetime);
            if (!ViewModel.IsCodeStep) timer.Stop();
        }
        catch (OperationCanceledException) { }
        // ! An async void handler that throws takes the app down, so any other failure is shown in the dialog instead.
        catch (Exception)
        {
            timer.Stop();
            ViewModel.FailUnexpectedly();
        }
    }

    private async void Dialog_Closed(ContentDialog sender, ContentDialogClosedEventArgs args)
    {
        closed = true;
        timer.Stop();
        // * Closing the view model cancels its running steps and waits for them. A failure here must not escape the
        // * async void handler and crash the app.
        try
        {
            await ViewModel.CloseAsync();
        }
        catch (Exception)
        {
            ViewModel.FailUnexpectedly();
        }
    }
}
