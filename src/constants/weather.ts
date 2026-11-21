import {
  mdiSunWirelessOutline,
  mdiWeatherCloudy,
  mdiWeatherDust,
  mdiWeatherFog,
  mdiWeatherHail,
  mdiWeatherHazy,
  mdiWeatherHurricane,
  mdiWeatherLightning,
  mdiWeatherLightningRainy,
  mdiWeatherPouring,
  mdiWeatherRainy,
  mdiWeatherSnowy,
  mdiWeatherSnowyRainy,
  mdiWeatherSunny,
  mdiWeatherSunnyOff,
  mdiWeatherTornado,
  mdiWeatherWindy,
} from "@mdi/js";
import type { MessageKey } from "../i18n";

export interface Weather {
  code: string;
  path: string;
  labelKey: MessageKey;
}

export const WEATHERS: Weather[] = [
  { code: "sunny", path: mdiWeatherSunny, labelKey: "weather.sunny" },
  { code: "sunny-off", path: mdiWeatherSunnyOff, labelKey: "weather.sunny-off" },
  { code: "cloudy", path: mdiWeatherCloudy, labelKey: "weather.cloudy" },
  { code: "hazy", path: mdiWeatherHazy, labelKey: "weather.hazy" },
  { code: "fog", path: mdiWeatherFog, labelKey: "weather.fog" },
  { code: "rainy", path: mdiWeatherRainy, labelKey: "weather.rainy" },
  { code: "pouring", path: mdiWeatherPouring, labelKey: "weather.pouring" },
  { code: "lightning", path: mdiWeatherLightning, labelKey: "weather.lightning" },
  {
    code: "lightning-rainy",
    path: mdiWeatherLightningRainy,
    labelKey: "weather.lightning-rainy",
  },
  { code: "sleet", path: mdiWeatherSnowyRainy, labelKey: "weather.sleet" },
  { code: "snow", path: mdiWeatherSnowy, labelKey: "weather.snow" },
  { code: "hail", path: mdiWeatherHail, labelKey: "weather.hail" },
  { code: "windy", path: mdiWeatherWindy, labelKey: "weather.windy" },
  { code: "dust", path: mdiWeatherDust, labelKey: "weather.dust" },
  { code: "tornado", path: mdiWeatherTornado, labelKey: "weather.tornado" },
  { code: "hurricane", path: mdiWeatherHurricane, labelKey: "weather.hurricane" },
  {
    code: "ultraviolet",
    path: mdiSunWirelessOutline,
    labelKey: "weather.ultraviolet",
  },
];

const BY_CODE = new Map(WEATHERS.map((weather) => [weather.code, weather]));

export function weatherByCode(code: string | null | undefined): Weather | undefined {
  return code ? BY_CODE.get(code) : undefined;
}