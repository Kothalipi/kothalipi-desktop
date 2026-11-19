/**
 * Language metadata for supported locales (Bangla & English).
 */
export const LANGUAGE_METADATA: Record<
  string,
  {
    name: string;
    nativeName: string;
    priority?: number;
    direction?: "ltr" | "rtl";
  }
> = {
  bn: { name: "Bangla", nativeName: "বাংলা", priority: 1 },
  en: { name: "English", nativeName: "English", priority: 2 },
};
