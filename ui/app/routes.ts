import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/_index.tsx"),
  route("mixer", "routes/mixer.tsx"),
  route("piano-roll", "routes/piano-roll.tsx"),
  route("timeline", "routes/timeline.tsx"),
  route("plugins", "routes/plugins.tsx"),
  route("effects", "routes/effects.tsx"),
] satisfies RouteConfig;
