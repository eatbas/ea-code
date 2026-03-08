/** Skill record managed by the local skills catalogue. */
export interface Skill {
  id: string;
  name: string;
  description: string;
  instructions: string;
  tags: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

/** Payload for creating a skill. */
export interface CreateSkillPayload {
  name: string;
  description: string;
  instructions: string;
  tags?: string;
  isActive?: boolean;
}

/** Payload for updating a skill. */
export interface UpdateSkillPayload {
  id: string;
  name?: string;
  description?: string;
  instructions?: string;
  tags?: string;
  isActive?: boolean;
}
