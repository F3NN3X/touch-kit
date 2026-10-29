using System.Runtime.InteropServices;

namespace TouchKit;

/// <summary>The C API in touch_kit.h. touch_kit.dll must sit next to the app (or on the DLL search path).</summary>
internal static class Native
{
    private const string Lib = "touch_kit";

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    internal delegate void Callback(IntPtr user, IntPtr json);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr tk_version();

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr tk_last_error();

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void tk_free(IntPtr s);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr tk_touchscreens();

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr tk_report(uint recordSeconds);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr tk_start(byte[] configJson, Callback callback, IntPtr user);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    internal static extern bool tk_reconfigure(IntPtr device, byte[] panelJson, byte[] gesturesJson);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void tk_stop(IntPtr device);

    /// <summary>NUL-terminated UTF-8, as the C API takes strings.</summary>
    internal static byte[] Utf8(string s) => System.Text.Encoding.UTF8.GetBytes(s + "\0");

    /// <summary>Reads and frees a string the library returned; null for a null pointer.</summary>
    internal static string? Take(IntPtr s)
    {
        if (s == IntPtr.Zero) return null;
        try { return Marshal.PtrToStringUTF8(s); }
        finally { tk_free(s); }
    }

    internal static TouchKitException LastError(string what) =>
        new($"{what}: {Take(tk_last_error()) ?? "unknown error"}");
}

/// <summary>A call into touch_kit.dll failed; the message is the library's reason.</summary>
public sealed class TouchKitException(string message) : Exception(message);
