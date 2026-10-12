<?php

declare(strict_types=1);

namespace LingaraEmbedSnippets;

// lingara:begin widgetSnippet
use Lingara\Embed\SnippetOptions;
use Lingara\Embed\WidgetSnippet;
// lingara:end
// lingara:begin widgetSnippet
// In your page's template: the plan your server made for this player, and
// your token route. Pass the request's CSP nonce if your script-src allows
// no inline script. A bad option throws SnippetException naming its field.
function practiceWidget(string $planId, string $cspNonce): string
{
    return WidgetSnippet::render(new SnippetOptions(
        planId: $planId,
        tokenEndpoint: '/lingara/token',
        theme: 'auto',
        nonce: $cspNonce,
    ));
}
// lingara:end