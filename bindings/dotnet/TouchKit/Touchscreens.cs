using System.Runtime.InteropServices;
using System.Text.Json;

namespace TouchKit;

public static class Touchscreens
{
    /// <summary>touch_kit.dll's version.</summary>
    public static string LibraryVersion => Marshal.PtrToStringUTF8(Native.tk_version())!;

    /// <summary>Every connected touchscreen, ready or not.</summary>
    public static IReadOnlyList<Touchscreen> List() =>
        JsonSerializer.Deserialize<List<Touchscreen>>(
            Native.Take(Native.tk_touchscreens()) ?? throw Native.LastError("listing touchscreens"),
            TouchConfig.Json)!;

    /// <summary>
    /// A display report as JSON (the touchscreens' USB and HID details), after recording
    /// <paramref name="recordSeconds"/> of touches on each ready one (0 = none). Blocks for the
    /// recording; dispose running <see cref="TouchDevice"/>s first, they hold the devices.
    /// </summary>
    public static string CreateReport(uint recordSeconds = 0) =>
        Native.Take(Native.tk_report(recordSeconds)) ?? throw Native.LastError("creating the report");
}
