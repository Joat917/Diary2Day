import {
  mdiEmoticonAngry,
  mdiEmoticonConfused,
  mdiEmoticonCool,
  mdiEmoticonCry,
  mdiEmoticonDead,
  mdiEmoticonDevil,
  mdiEmoticonExcited,
  mdiEmoticonFrown,
  mdiEmoticonHappy,
  mdiEmoticonKiss,
  mdiEmoticonLol,
  mdiEmoticonNeutral,
  mdiEmoticonPoop,
  mdiEmoticonSad,
  mdiEmoticonSick,
  mdiEmoticonTongue,
  mdiEmoticonWink,
} from "@mdi/js";
import type { MessageKey } from "../i18n";

export interface Mood {
  code: string;
  path: string;
  labelKey: MessageKey;
}

export const MOODS: Mood[] = [
  { code: "angry", path: mdiEmoticonAngry, labelKey: "mood.angry" },
  { code: "confused", path: mdiEmoticonConfused, labelKey: "mood.confused" },
  { code: "cool", path: mdiEmoticonCool, labelKey: "mood.cool" },
  { code: "cry", path: mdiEmoticonCry, labelKey: "mood.cry" },
  { code: "dead", path: mdiEmoticonDead, labelKey: "mood.dead" },
  { code: "devil", path: mdiEmoticonDevil, labelKey: "mood.devil" },
  { code: "excited", path: mdiEmoticonExcited, labelKey: "mood.excited" },
  { code: "frown", path: mdiEmoticonFrown, labelKey: "mood.frown" },
  { code: "happy", path: mdiEmoticonHappy, labelKey: "mood.happy" },
  { code: "kiss", path: mdiEmoticonKiss, labelKey: "mood.kiss" },
  { code: "lol", path: mdiEmoticonLol, labelKey: "mood.lol" },
  { code: "neutral", path: mdiEmoticonNeutral, labelKey: "mood.neutral" },
  { code: "poop", path: mdiEmoticonPoop, labelKey: "mood.poop" },
  { code: "sad", path: mdiEmoticonSad, labelKey: "mood.sad" },
  { code: "sick", path: mdiEmoticonSick, labelKey: "mood.sick" },
  { code: "tongue", path: mdiEmoticonTongue, labelKey: "mood.tongue" },
  { code: "wink", path: mdiEmoticonWink, labelKey: "mood.wink" },
];

const BY_CODE = new Map(MOODS.map((mood) => [mood.code, mood]));

export function moodByCode(code: string | null | undefined): Mood | undefined {
  return code ? BY_CODE.get(code) : undefined;
}