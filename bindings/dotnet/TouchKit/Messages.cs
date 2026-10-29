using System.Text.Json;

namespace TouchKit;

public enum PointerKind { Down, Move, Up }

/// <summary>One contact event in panel px (see <see cref="PanelConfig"/>); <c>Id</c> follows a finger from Down to Up.</summary>
public sealed record PointerEvent(PointerKind Kind, uint Id, float X, float Y, ulong TMs);

/// <summary>
/// A recognised gesture at a point (its start, or the centroid for multi-finger gestures).
/// <c>Type</c> is Tap, DoubleTap, LongPress, LongPressRepeat (<c>N</c>), DragStart, DragMove
/// (<c>Dx</c>/<c>Dy</c> from the start), DragEnd, Swipe (<c>Dir</c>: Left/Right/Up/Down, <c>Fingers</c>),
/// PinchStart, Pinch (<c>Scale</c>), PinchEnd or ThreeFingerTap.
/// </summary>
public sealed record GestureEvent(
    string Type, float X, float Y, ulong TMs,
    float? Dx = null, float? Dy = null, string? Dir = null, int? Fingers = null, float? Scale = null, int? N = null);

public enum DeviceState { Connected, Disconnected, Error }

/// <summary>Connection state; <c>Port</c> and <c>ReportLen</c> when connected, <c>Message</c> on an error.</summary>
public sealed record DeviceStatus(DeviceState State, string? Port = null, int? ReportLen = null, string? Message = null);

/// <summary>A connected touchscreen. <c>Ready</c> false: its driver setup is still to do (docs/driver-setup.md).</summary>
public sealed record Touchscreen(ushort Vid, ushort Pid, string Port, string? Serial, string? Product, string? Driver, bool Ready);

internal static class Messages
{
    /// <summary>Parses one message from the library into a status, pointer or gesture; null for others.</summary>
    internal static object? Parse(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var m = doc.RootElement;
        if (m.TryGetProperty("status", out var status))
        {
            return new DeviceStatus(
                Enum.Parse<DeviceState>(status.GetString()!),
                Str(m, "port"), Int(m, "report_len"), Str(m, "message"));
        }
        switch (Str(m, "type"))
        {
            case "Pointer":
                return new PointerEvent(
                    Enum.Parse<PointerKind>(Str(m, "kind")!), m.GetProperty("id").GetUInt32(),
                    m.GetProperty("x").GetSingle(), m.GetProperty("y").GetSingle(), m.GetProperty("t_ms").GetUInt64());
            case "Gesture":
                var g = m.GetProperty("gesture");
                return new GestureEvent(
                    Str(g, "type")!, m.GetProperty("x").GetSingle(), m.GetProperty("y").GetSingle(), m.GetProperty("t_ms").GetUInt64(),
                    Float(g, "dx"), Float(g, "dy"), Str(g, "dir"), Int(g, "fingers"), Float(g, "scale"), Int(g, "n"));
            default:
                return null; // "Ignored": a non-touch report id, nothing to act on
        }
    }

    private static string? Str(JsonElement e, string k) => e.TryGetProperty(k, out var v) ? v.GetString() : null;
    private static int? Int(JsonElement e, string k) => e.TryGetProperty(k, out var v) ? v.GetInt32() : null;
    private static float? Float(JsonElement e, string k) => e.TryGetProperty(k, out var v) ? v.GetSingle() : null;
}
