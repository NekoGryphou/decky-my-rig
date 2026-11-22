using Microsoft.UI.Xaml;
using Windows.Win32;
using Windows.Win32.Foundation;

namespace MyRigConfig.Shell;

internal static class ProgramNative
{
    /// <summary>Lets the running instance take the foreground this launch was given.</summary>
    internal static void AllowForeground(uint processId) => WindowPInvoke.AllowSetForegroundWindow(processId);

    internal static void BringToFront(Window window)
    {
        window.AppWindow.Show();
        window.Activate();
        WindowPInvoke.SetForegroundWindow(new HWND(WinRT.Interop.WindowNative.GetWindowHandle(window)));
    }
}
