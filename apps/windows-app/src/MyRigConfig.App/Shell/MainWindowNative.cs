using Windows.Win32;
using Windows.Win32.Foundation;

namespace MyRigConfig.Shell;

public sealed partial class MainWindow
{
    private static uint GetDpiForWindow(nint window) => WindowPInvoke.GetDpiForWindow(new HWND(window));
}
