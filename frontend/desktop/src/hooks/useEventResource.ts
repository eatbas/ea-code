import { useState, type Dispatch, type SetStateAction } from "react";
import { useTauriEventListeners } from "./useTauriEventListeners";

export function upsertByKey<T, K>(items: readonly T[], next: T, getKey: (value: T) => K): T[] {
  const nextKey = getKey(next);
  return [...items.filter((item) => getKey(item) !== nextKey), next];
}

export function assignByKey<T, K extends string>(
  record: Readonly<Record<K, T>>,
  next: T,
  getKey: (value: T) => K,
): Record<K, T> {
  return {
    ...record,
    [getKey(next)]: next,
  };
}

interface UseEventStateOptions<S, T> {
  initialState: S;
  itemEvent: string;
  doneEvent?: string;
  reduce: (state: S, payload: T) => S;
}

interface EventStateResource<S> {
  state: S;
  setState: Dispatch<SetStateAction<S>>;
  loading: boolean;
  setLoading: (value: boolean) => void;
}

export function useEventState<S, T>(options: UseEventStateOptions<S, T>): EventStateResource<S> {
  const [state, setState] = useState(options.initialState);
  const { checking, setChecking } = useTauriEventListeners({
    listeners: [
      {
        event: options.itemEvent,
        handler: (payload: T) => {
          setState((previous) => options.reduce(previous, payload));
        },
      },
    ],
    doneEvent: options.doneEvent,
  });

  return {
    state,
    setState,
    loading: checking,
    setLoading: setChecking,
  };
}

interface UseEventValueOptions<T> {
  initialValue: T;
  itemEvent: string;
  doneEvent?: string;
}

export function useEventValue<T>(options: UseEventValueOptions<T>): EventStateResource<T> {
  return useEventState<T, T>({
    initialState: options.initialValue,
    itemEvent: options.itemEvent,
    doneEvent: options.doneEvent,
    reduce: (_previous, payload) => payload,
  });
}

interface UseEventListOptions<T, K> {
  initialState?: T[];
  itemEvent: string;
  doneEvent?: string;
  getKey: (value: T) => K;
}

export function useEventList<T, K>(options: UseEventListOptions<T, K>): EventStateResource<T[]> {
  return useEventState<T[], T>({
    initialState: options.initialState ?? [],
    itemEvent: options.itemEvent,
    doneEvent: options.doneEvent,
    reduce: (items, payload) => upsertByKey(items, payload, options.getKey),
  });
}

interface UseEventRecordOptions<V, P, K extends string, S extends Record<K, V>> {
  initialState: S;
  itemEvent: string;
  doneEvent?: string;
  getKey: (payload: P) => K;
  getValue: (payload: P) => V;
}

export function useEventRecord<V, P, K extends string, S extends Record<K, V>>(
  options: UseEventRecordOptions<V, P, K, S>,
): EventStateResource<S> {
  return useEventState<S, P>({
    initialState: options.initialState,
    itemEvent: options.itemEvent,
    doneEvent: options.doneEvent,
    reduce: (record, payload) => ({
      ...record,
      [options.getKey(payload)]: options.getValue(payload),
    }),
  });
}
