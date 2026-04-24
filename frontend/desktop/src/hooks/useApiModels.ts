import { useCallback } from "react";
import type { ModelDetail } from "../types";
import { API_EVENTS } from "../constants/events";
import { getApiModels } from "../lib/desktopApi";
import { useEventList } from "./useEventResource";

interface UseApiModelsReturn {
  models: ModelDetail[];
  loading: boolean;
  fetchModels: () => void;
}

/** Hook to fetch Symphony model details with per-model option schemas. */
export function useApiModels(): UseApiModelsReturn {
  const {
    state: models,
    loading,
    setLoading,
  } = useEventList<ModelDetail, string>({
    itemEvent: API_EVENTS.MODEL_INFO,
    doneEvent: API_EVENTS.MODELS_COMPLETE,
    getKey: (model) => `${model.provider}:${model.model}`,
  });

  const fetchModels = useCallback((): void => {
    setLoading(true);
    getApiModels().catch(() => {
      setLoading(false);
    });
  }, [setLoading]);

  return { models, loading, fetchModels };
}
