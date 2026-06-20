export interface Account {
  id: string;
  phone: string;
  label: string;
  user_id?: string;
  saved: boolean;
}

export interface AccountWithStatus extends Account {
  is_current: boolean;
}

export interface ProgressEvent {
  step: string;
  current: number;
  total: number;
}
