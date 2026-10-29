import { useEffect } from "react";
import { Calibrate } from "./components/Calibrate";
import { Identify } from "./components/Identify";
import { StatusStrip } from "./components/StatusStrip";
import { TouchDots } from "./kit/TouchDots";
import { ToastHost } from "./kit/toast";
import { TouchKitScreen } from "./screens/TouchKitScreen";
import { startActions } from "./touch/actions";
import { onDeviceMessage } from "./touch/bridge";
import { startErrorReporting } from "./touch/errors";
import { startSettings } from "./touch/settings";

export default function App() {
  useEffect(() => {
    startErrorReporting();
    startSettings();
    startActions();
    // subscribe to the bridge even before any component listens
    return onDeviceMessage(() => {});
  }, []);

  return (
    <main className="relative h-full w-full">
      <TouchKitScreen />
      <StatusStrip />
      <ToastHost />
      <TouchDots />
      <Identify />
      <Calibrate />
    </main>
  );
}
