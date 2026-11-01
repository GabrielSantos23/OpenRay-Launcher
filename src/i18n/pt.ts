import { ptCore } from "./pt-core";
import { ptExtensionsA } from "./pt-extensions-a";
import { ptExtensionsB } from "./pt-extensions-b";
import { ptFlowkey } from "./pt-flowkey";
import { ptSettings } from "./pt-settings";
import { ptWindows } from "./pt-windows";

export const pt: Record<string, string> = Object.freeze({ ...ptCore, ...ptSettings, ...ptWindows, ...ptExtensionsA, ...ptExtensionsB, ...ptFlowkey });
