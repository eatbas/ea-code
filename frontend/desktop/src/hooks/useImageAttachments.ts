import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { saveConversationImage } from "../lib/desktopApi";
import { blobToBase64, buildPromptWithImages as formatPromptWithImages } from "../utils/imageUtils";

/** An image that has been saved to disk. */
export interface AttachedImage {
  fileName: string;
  filePath: string;
  previewUrl: string;
}

/** An image buffered in memory, waiting for a conversationId. */
export interface PendingImage {
  blob: Blob;
  extension: string;
  previewUrl: string;
}

const MIME_TO_EXTENSION: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
};

const MAX_IMAGE_SIZE = 20 * 1024 * 1024; // 20 MB

export function useImageAttachments(
  workspacePath: string | null,
  conversationId: string | null,
) {
  const [attachedImages, setAttachedImages] = useState<AttachedImage[]>([]);
  const [pendingImages, setPendingImages] = useState<PendingImage[]>([]);

  // Track blob URLs for cleanup on unmount (asset URLs do not need revoking).
  const blobUrlsRef = useRef<Set<string>>(new Set());

  // Clear attachments when the active conversation changes — images from prior
  // turns are already embedded in the conversation history and should not be
  // re-sent.
  const prevConversationIdRef = useRef<string | null>(conversationId);
  useEffect(() => {
    if (prevConversationIdRef.current === conversationId) {
      return;
    }

    // Clean up blob URLs from the previous conversation.
    blobUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    blobUrlsRef.current.clear();

    prevConversationIdRef.current = conversationId;
    setAttachedImages([]);
    setPendingImages([]);
  }, [conversationId]);

  // Revoke all remaining blob URLs on unmount.
  useEffect(() => {
    return () => {
      blobUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    };
  }, []);

  const addImages = useCallback(async (files: File[]): Promise<void> => {
    for (const file of files) {
      const extension = MIME_TO_EXTENSION[file.type];
      if (!extension) continue;

      if (file.size > MAX_IMAGE_SIZE) {
        console.warn(`[useImageAttachments] Image too large (${file.size} bytes), skipping.`);
        continue;
      }

      const previewUrl = URL.createObjectURL(file);
      blobUrlsRef.current.add(previewUrl);

      if (workspacePath && conversationId) {
        try {
          const base64 = await blobToBase64(file);
          const result = await saveConversationImage(workspacePath, conversationId, base64, extension);
          // Replace the blob URL with a stable asset URL for the saved file.
          URL.revokeObjectURL(previewUrl);
          blobUrlsRef.current.delete(previewUrl);
          const assetUrl = convertFileSrc(result.filePath);
          setAttachedImages((prev) => [
            ...prev,
            { fileName: result.fileName, filePath: result.filePath, previewUrl: assetUrl },
          ]);
        } catch (error) {
          console.warn("[useImageAttachments] Failed to save image:", error);
          URL.revokeObjectURL(previewUrl);
          blobUrlsRef.current.delete(previewUrl);
        }
      } else {
        setPendingImages((prev) => [
          ...prev,
          { blob: file, extension, previewUrl },
        ]);
      }
    }
  }, [workspacePath, conversationId]);

  const revokeBlobUrl = useCallback((url: string) => {
    if (blobUrlsRef.current.has(url)) {
      URL.revokeObjectURL(url);
      blobUrlsRef.current.delete(url);
    }
  }, []);

  const removeImage = useCallback((index: number) => {
    const totalAttached = attachedImages.length;
    if (index < totalAttached) {
      setAttachedImages((prev) => {
        const updated = [...prev];
        const removed = updated.splice(index, 1);
        removed.forEach((img) => revokeBlobUrl(img.previewUrl));
        return updated;
      });
    } else {
      const pendingIndex = index - totalAttached;
      setPendingImages((prev) => {
        const updated = [...prev];
        const removed = updated.splice(pendingIndex, 1);
        removed.forEach((img) => revokeBlobUrl(img.previewUrl));
        return updated;
      });
    }
  }, [attachedImages.length, revokeBlobUrl]);

  const clearImages = useCallback(() => {
    setAttachedImages((prev) => {
      prev.forEach((img) => revokeBlobUrl(img.previewUrl));
      return [];
    });
    setPendingImages((prev) => {
      prev.forEach((img) => revokeBlobUrl(img.previewUrl));
      return [];
    });
  }, [revokeBlobUrl]);

  const buildPromptWithImages = useCallback((prompt: string, extraPaths?: string[]): string => {
    const paths = [
      ...attachedImages.map((img) => img.filePath),
      ...(extraPaths ?? []),
    ];
    return formatPromptWithImages(prompt, paths);
  }, [attachedImages]);

  const allPreviews = useMemo(() => [
    ...attachedImages.map((img) => ({ previewUrl: img.previewUrl, label: img.fileName })),
    ...pendingImages.map((img, i) => ({ previewUrl: img.previewUrl, label: `Pending image ${i + 1}` })),
  ], [attachedImages, pendingImages]);

  const hasImages = useMemo(
    () => attachedImages.length > 0 || pendingImages.length > 0,
    [attachedImages, pendingImages],
  );

  return {
    attachedImages,
    pendingImages,
    hasImages,
    allPreviews,
    addImages,
    removeImage,
    clearImages,
    buildPromptWithImages,
  };
}
