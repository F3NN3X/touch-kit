// Lists touchscreens, then prints touches and gestures until Enter.
//   dotnet run --project bindings/dotnet/TouchKit.Sample               the first touchscreen found
//   dotnet run --project bindings/dotnet/TouchKit.Sample -- tap.jsonl  a recording, no hardware needed
using TouchKit;

Console.WriteLine($"touch_kit {Touchscreens.LibraryVersion}");
foreach (var t in Touchscreens.List())
    Console.WriteLine($"  {t.Vid:x4}:{t.Pid:x4} {t.Product} at {t.Port}{(t.Ready ? "" : " (driver setup needed)")}");

var config = new TouchConfig
{
    Panel = new PanelConfig { WidthPx = 1920, HeightPx = 550 },
    Replay = args.FirstOrDefault(),
};
using var device = new TouchDevice(config);
device.Status += s => Console.WriteLine($"status {s.State} {s.Port}{s.Message}");
device.Pointer += p => { if (p.Kind != PointerKind.Move) Console.WriteLine($"pointer {p.Kind} #{p.Id} {p.X:0},{p.Y:0}"); };
device.Gesture += g => Console.WriteLine($"gesture {g.Type} {g.Dir} at {g.X:0},{g.Y:0}");

Console.WriteLine("Touch the screen; Enter quits.");
Console.ReadLine();
