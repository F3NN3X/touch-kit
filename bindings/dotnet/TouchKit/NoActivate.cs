using System.Runtime.InteropServices;

namespace TouchKit;

/// <summary>
/// Makes a window that is never activated: touching it doesn't take focus from the app the user is
/// typing in, and it stays out of the taskbar and Alt-Tab. Call it once the window has a handle; in
/// WPF from <c>SourceInitialized</c> with <c>new WindowInteropHelper(this).Handle</c> (and set
/// <c>ShowActivated="False"</c>), in WinForms from <c>OnHandleCreated</c> with <c>Handle</c>.
/// </summary>
public static class NoActivate
{
    private const int GWL_EXSTYLE = -20;
    private const long WS_EX_NOACTIVATE = 0x0800_0000, WS_EX_TOOLWINDOW = 0x80, WS_EX_APPWINDOW = 0x4_0000;
    private const uint SWP_NOSIZE = 0x1, SWP_NOMOVE = 0x2, SWP_NOZORDER = 0x4, SWP_NOACTIVATE = 0x10, SWP_FRAMECHANGED = 0x20;

    public static void Apply(IntPtr hwnd)
    {
        var ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE).ToInt64();
        ex = (ex | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) & ~WS_EX_APPWINDOW;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new IntPtr(ex));
        SetWindowPos(hwnd, IntPtr.Zero, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    }

    [DllImport("user32.dll")] private static extern IntPtr GetWindowLongPtrW(IntPtr hwnd, int index);
    [DllImport("user32.dll")] private static extern IntPtr SetWindowLongPtrW(IntPtr hwnd, int index, IntPtr value);
    [DllImport("user32.dll")]
    private static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
}
