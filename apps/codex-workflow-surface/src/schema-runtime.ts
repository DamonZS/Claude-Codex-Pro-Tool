import { z } from "zod";

// Configure before importing schemas: Codex's CSP blocks Function compilation.
z.config({ jitless: true });
