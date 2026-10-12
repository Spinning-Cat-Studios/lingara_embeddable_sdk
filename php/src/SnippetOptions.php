<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * What WidgetSnippet::render() writes (ADR 1.10.26af D7). `planId` and
 * `tokenEndpoint` are required; every other option is written only when
 * given. Pass them by name:
 *
 *     new SnippetOptions(planId: $plan->id, tokenEndpoint: '/lingara/token', nonce: $cspNonce)
 */
final readonly class SnippetOptions
{
    /**
     * @param string      $planId        the lesson plan your server made for this player, a UUID
     * @param string      $tokenEndpoint your token route, a path (`/lingara/token`) or an `https://`
     *                                   URL; the element POSTs to it with the page's cookies
     * @param string|null $surface       default `practice`
     * @param string|null $theme         `light`, `dark` or `auto`
     * @param string|null $id            default `lingara-embed`; two widgets on one page need two ids
     * @param string|null $nonce         your page's CSP nonce, written on both scripts
     * @param string|null $elementSrc    default the CDN build at this kit's Version::VERSION
     */
    public function __construct(
        public string $planId,
        public string $tokenEndpoint,
        public ?string $surface = null,
        public ?string $theme = null,
        public ?string $locale = null,
        public ?string $lingaraOrigin = null,
        public ?string $title = null,
        public ?string $id = null,
        public ?string $nonce = null,
        public ?string $elementSrc = null,
    ) {}
}
