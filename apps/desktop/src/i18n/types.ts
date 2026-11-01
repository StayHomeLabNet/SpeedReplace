export type TranslationParams = Record<string, string | number>;

export interface LocaleDefinition<Key extends string = string> {
  code: string;
  name: string;
  messages: Record<Key, string>;
}
