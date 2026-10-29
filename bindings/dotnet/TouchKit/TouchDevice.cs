using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace TouchKit;

/// <summary>
/// Reads one touchscreen until disposed, reconnecting by itself when it is unplugged.
/// <para>
/// Events are raised on the <see cref="SynchronizationContext"/> that was current at construction (the
/// UI thread in WPF and WinForms), otherwise on a library thread; a console app gets the latter and
/// must not call <see cref="Dispose"/> from inside a handler.
/// </para>
/// </summary>
public sealed class TouchDevice : IDisposable
{
    private readonly Native.Callback _callback; // held so the GC can't collect it while native code calls it
    private readonly SynchronizationContext? _context = SynchronizationContext.Current;
    private IntPtr _handle;
    private volatile bool _disposed;

    public event Action<DeviceStatus>? Status;
    public event Action<PointerEvent>? Pointer;
    public event Action<GestureEvent>? Gesture;

    /// <summary>
    /// Starts reading. With a <see cref="SynchronizationContext"/> nothing is raised before the
    /// constructor's caller returns to it, so handlers added right after see every message; without one,
    /// the first status can arrive before they are added.
    /// </summary>
    /// <exception cref="TouchKitException">The config is invalid.</exception>
    public TouchDevice(TouchConfig? config = null)
    {
        _callback = OnMessage;
        var json = JsonSerializer.Serialize(config ?? new TouchConfig(), TouchConfig.Json);
        _handle = Native.tk_start(Native.Utf8(json), _callback, IntPtr.Zero);
        if (_handle == IntPtr.Zero) throw Native.LastError("starting the touch device");
    }

    /// <summary>Applies a new panel mapping and gesture thresholds without reopening the device (null = defaults).</summary>
    public void Reconfigure(PanelConfig? panel, GestureConfig? gestures)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        var ok = Native.tk_reconfigure(
            _handle,
            Native.Utf8(JsonSerializer.Serialize(panel ?? new PanelConfig(), TouchConfig.Json)),
            Native.Utf8(JsonSerializer.Serialize(gestures ?? new GestureConfig(), TouchConfig.Json)));
        if (!ok) throw Native.LastError("reconfiguring");
    }

    /// <summary>Stops reading and releases the device; no event is raised after this returns.</summary>
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        Native.tk_stop(_handle); // joins the callback thread
        _handle = IntPtr.Zero;
        GC.KeepAlive(_callback);
    }

    private void OnMessage(IntPtr user, IntPtr json)
    {
        // An exception must not unwind into native code: it would take the process down.
        try
        {
            var message = Messages.Parse(Marshal.PtrToStringUTF8(json)!);
            if (message is null) return;
            if (_context is null) Raise(message);
            else _context.Post(m => Raise(m!), message);
        }
        catch (Exception e)
        {
            Trace.TraceError($"touch-kit: {e}");
        }
    }

    private void Raise(object message)
    {
        if (_disposed) return; // posted before Dispose, delivered after
        switch (message)
        {
            case DeviceStatus s: Status?.Invoke(s); break;
            case PointerEvent p: Pointer?.Invoke(p); break;
            case GestureEvent g: Gesture?.Invoke(g); break;
        }
    }
}
