using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;

namespace MyRigConfig.Shell;

/// <summary>Starts the app once. A second launch brings the running window to the front and exits,
/// like Windows Settings does.</summary>
public static class Program
{
    [STAThread]
    private static int Main()
    {
        WinRT.ComWrappersSupport.InitializeComWrappers();
        var instance = AppInstance.FindOrRegisterForKey(ProgramConstants.InstanceKey);
        if (!instance.IsCurrent)
        {
            Redirect(instance);
            return 0;
        }
        Application.Start(initialization =>
        {
            SynchronizationContext.SetSynchronizationContext(
                new DispatcherQueueSynchronizationContext(DispatcherQueue.GetForCurrentThread()));
            _ = new App();
        });
        return 0;
    }

    /// <summary>Hands this launch over to the running instance. The redirect runs on a thread-pool
    /// thread and doesn't touch anything on this UI thread, which hasn't started yet.</summary>
    private static void Redirect(AppInstance running)
    {
        // ? This launch has foreground rights, so pass them on to the running window.
        ProgramNative.AllowForeground(running.ProcessId);
        var arguments = AppInstance.GetCurrent().GetActivatedEventArgs();
        Task.Run(() => running.RedirectActivationToAsync(arguments).AsTask()).Wait();
    }
}
