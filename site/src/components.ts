/**
 * MDX globals registry — components available inside MDX without `import`.
 * Wired via `<Content components={components} />` in `[...slug].astro`.
 *
 * Everything here is a page-authoring style: callouts, steps, tabs, cards.
 * The pages are `.mdx`, so these can be written in the body directly.
 */

import { Aside } from "./components/ui/aside";
import { Badge } from "./components/ui/badge";
import { Banner } from "./components/ui/banner";
import { Card } from "./components/ui/card";
import { CardGrid } from "./components/ui/card-grid";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "./components/ui/collapsible";
import { FileTree } from "./components/ui/file-tree";
import { Frame } from "./components/ui/frame";
import { LayerCard, LayerCardContent, LayerCardHeader } from "./components/ui/layer-card";
import { LinkButton } from "./components/ui/link-button";
import { LinkCard } from "./components/ui/link-card";
import { PackageManagers } from "./components/ui/package-managers";
import { Step, Steps } from "./components/ui/steps";
import { TabItem, Tabs } from "./components/ui/tabs";
import Render from "./components/Render.astro";

export const components = {
  Aside,
  Badge,
  Banner,
  Card,
  CardGrid,
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
  FileTree,
  Frame,
  LayerCard,
  LayerCardContent,
  LayerCardHeader,
  LinkButton,
  LinkCard,
  PackageManagers,
  Render,
  Step,
  Steps,
  TabItem,
  Tabs,
};