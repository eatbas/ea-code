const CHUNK_SIZE = 8192;

export async function blobToBase64(blob: Blob): Promise<string> {
  const buffer = await blob.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  const parts: string[] = [];
  for (let i = 0; i < bytes.length; i += CHUNK_SIZE) {
    parts.push(String.fromCharCode(...bytes.subarray(i, i + CHUNK_SIZE)));
  }
  return btoa(parts.join(""));
}

export function buildPromptWithImages(prompt: string, paths: string[]): string {
  if (paths.length === 0) return prompt;
  const imageList = paths.map((p) => `- ${p}`).join("\n");
  return `${prompt}\n\n[Attached Images]\n${imageList}`;
}
