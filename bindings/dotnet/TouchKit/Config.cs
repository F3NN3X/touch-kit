using System.Text.Json;
using System.Text.Json.Serialization;

namespace TouchKit;

/// <summary>
/// Which touchscreen to read, the panel it maps onto and the gesture thresholds. Leave a property null
/// for touch-core's default (see docs/config.md): <c>new TouchConfig()</c> is any touchscreen on a
/// 1920 × 550 panel.
/// </summary>
public sealed record TouchConfig
{
    public DeviceConfig? Device { get; init; }
    public PanelConfig? Panel { get; init; }
    public GestureConfig? Gestures { get; init; }
    /// <summary>Ports (<see cref="Touchscreen.Port"/>) other devices in this app use; never opened.</summary>
    public IReadOnlyList<string>? Taken { get; init; }
    /// <summary>A <c>touchprobe record</c> file to play in a loop instead of a USB device, for testing without hardware.</summary>
    public string? Replay { get; init; }

    internal static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };
}

public sealed record DeviceConfig
{
    public ushort? Vid { get; init; }
    public ushort? Pid { get; init; }
    public byte? Iface { get; init; }
    /// <summary>USB location from <see cref="Touchscreens.List"/>, to pick one of several identical panels.</summary>
    public string? Port { get; init; }
    public string? DescriptorFile { get; init; }
}

public sealed record PanelConfig
{
    /// <summary>The panel's size in physical pixels; touches arrive in this space.</summary>
    public uint? WidthPx { get; init; }
    public uint? HeightPx { get; init; }
    /// <summary>Clockwise rotation of touch coordinates: 0, 90, 180 or 270.</summary>
    [JsonIgnore] public int? RotationDegrees { get; init; }
    [JsonInclude, JsonPropertyName("rotation")] internal string? Rotation => RotationDegrees?.ToString();
    public bool? FlipX { get; init; }
    public bool? FlipY { get; init; }
}

/// <summary>Gesture thresholds in panel px and ms; null keeps the Touch Kit spec value.</summary>
public sealed record GestureConfig
{
    public float? SlopPx { get; init; }
    public ulong? LongPressMs { get; init; }
    public ulong? LongPressRepeatMs { get; init; }
    public ulong? DoubleTapGapMs { get; init; }
    public float? DoubleTapSlopPx { get; init; }
    public ulong? LiftDebounceMs { get; init; }
    public float? LiftDebouncePx { get; init; }
    public float? SwipeMinPx { get; init; }
    public float? FlickMinPx { get; init; }
    public float? FlickMinSpeed { get; init; }
    public float? SwipeAxisRatio { get; init; }
    public ulong? MultiFingerWindowMs { get; init; }
    public float? PinchMinScale { get; init; }
    public ulong? ThreeTapMaxMs { get; init; }
}
