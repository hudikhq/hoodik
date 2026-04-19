/**
 * Registers additional languages with refractor for Prism syntax highlighting.
 *
 * The default @milkdown/plugin-prism bundle only includes refractor's "common"
 * set (~38 languages). This plugin adds languages relevant to Hoodik's
 * user base (self-hosted / developer-heavy).
 */
import type { Ctx } from '@milkdown/ctx'
import { prismConfig } from '@milkdown/plugin-prism'

import dart from 'refractor/dart'
import docker from 'refractor/docker'
import elixir from 'refractor/elixir'
import graphql from 'refractor/graphql'
import hcl from 'refractor/hcl'
import kotlin from 'refractor/kotlin'
import nginx from 'refractor/nginx'
import protobuf from 'refractor/protobuf'
import swift from 'refractor/swift'
import toml from 'refractor/toml'
import zig from 'refractor/zig'

export function configurePrismLanguages(ctx: Ctx): void {
  ctx.update(prismConfig.key, (prev) => ({
    ...prev,
    configureRefractor: (refractor) => {
      refractor.register(dart)
      refractor.register(docker)
      refractor.register(elixir)
      refractor.register(graphql)
      refractor.register(hcl)
      refractor.register(kotlin)
      refractor.register(nginx)
      refractor.register(protobuf)
      refractor.register(swift)
      refractor.register(toml)
      refractor.register(zig)
    },
  }))
}
