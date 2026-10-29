// The app's touch bridge: @touch-kit/web, with failures in the status strip and latency in the
// debug stats. Import touch input from here so that wiring is in place first.
import { configureBridge } from "@touch-kit/web";
import { reportError } from "./errors";
import { record } from "./latency";

configureBridge({ onError: reportError, onLatency: record });

export * from "@touch-kit/web";
