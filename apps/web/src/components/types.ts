export type Me = {
  login: string;
  name?: string | null;
};

export type Project = {
  id: string;
  slug: string;
  name: string;
};

export type ApiKey = {
  id: string;
  key_prefix: string;
  name: string;
};

export type SecretListItem = {
  key_name: string;
  version: number;
};

export type SecretValue = {
  key_name: string;
  value: string;
};

export type MessageState = {
  text: string;
  isError: boolean;
};
