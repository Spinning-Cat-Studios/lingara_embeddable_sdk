<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * The first snippet option refused (ADR 1.10.26af D7, ES4), named by its
 * camelCase contract name in `field`: `planId`, `surface`, `theme`,
 * `locale`, `lingaraOrigin`, `title`, `id`, `tokenEndpoint`, `nonce` or
 * `elementSrc`. The message never carries the value.
 */
final class SnippetException extends \InvalidArgumentException
{
    /** @internal thrown by WidgetSnippet */
    public function __construct(
        public readonly string $field,
    ) {
        parent::__construct("the widget snippet's {$field} is not valid");
    }
}
