// @envfish/ui — shadcn/ui-style primitives shared by EnvFish frontends.
// Components are copied-in (not a dependency) so they can be tuned freely.
export { cn } from "./lib/cn";
export { Button, buttonVariants } from "./components/button";
export type { ButtonProps } from "./components/button";
export { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "./components/card";
export { Input } from "./components/input";
export { Label } from "./components/label";
export { Badge, badgeVariants } from "./components/badge";
export { Goldfish } from "./components/goldfish";
export type { GoldfishVariant } from "./components/goldfish";
export { GoldfishLoader, GoldfishInline } from "./components/goldfish-loader";
