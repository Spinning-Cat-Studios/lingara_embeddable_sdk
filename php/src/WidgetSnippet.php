<?php

declare(strict_types=1);

namespace Lingara\Embed;

/**
 * The widget snippet (ADR 1.10.26af D7, ES4): the `<lingara-embed>` element
 * and its two module scripts, rendered identically by all eight server
 * kits. The token never appears in it, only the endpoint the element asks
 * at run time, with `credentials: "same-origin"`.
 *
 *     echo WidgetSnippet::render(new SnippetOptions(planId: $planId, tokenEndpoint: '/lingara/token'));
 */
final class WidgetSnippet
{
    private const ATTRIBUTE = ['&' => '&amp;', '<' => '&lt;', '>' => '&gt;', '"' => '&quot;', "'" => '&#39;'];
    private const SCRIPT = ['<' => '\u003c', '>' => '\u003e', '&' => '\u0026', "\u{2028}" => '\u2028', "\u{2029}" => '\u2029'];
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_LINE_TERMINATORS | JSON_THROW_ON_ERROR;
    private const SCRIPT_BODY = '.tokenProvider = async () => (await fetch(%s, { method: "POST", credentials: "same-origin" })).json();';

    /** The script-tag build of this release's element. */
    public static function defaultElementSrc(): string
    {
        return 'https://cdn.jsdelivr.net/npm/@lingara/embed@' . Version::VERSION . '/dist/lingara-embed.min.js';
    }

    /**
     * The three lines to paste into your page, with no trailing newline.
     *
     * @throws SnippetException naming the first refused option
     */
    public static function render(SnippetOptions $o): string
    {
        $surface = $o->surface ?? 'practice';
        $id = $o->id ?? 'lingara-embed';
        $src = $o->elementSrc ?? self::defaultElementSrc();
        self::check($o, $surface, $id);
        $element = '<lingara-embed id="' . self::attr($id) . '" surface="' . self::attr($surface) . '" plan-id="' . self::attr($o->planId) . '"';
        $optional = ['theme' => $o->theme, 'locale' => $o->locale, 'lingara-origin' => $o->lingaraOrigin, 'title' => $o->title];
        foreach ($optional as $name => $value) {
            $element .= $value === null ? '' : " {$name}=\"" . self::attr($value) . '"';
        }
        $nonce = $o->nonce === null ? '' : ' nonce="' . self::attr($o->nonce) . '"';
        $script = 'document.getElementById(' . self::scriptString($id) . ')' . sprintf(self::SCRIPT_BODY, self::scriptString($o->tokenEndpoint));
        return "{$element}></lingara-embed>\n"
            . "<script type=\"module\"{$nonce} src=\"" . self::attr($src) . "\"></script>\n"
            . "<script type=\"module\"{$nonce}>{$script}</script>";
    }

    /** Each rule in the contract's order; the first failure names its option. */
    private static function check(SnippetOptions $o, string $surface, string $id): void
    {
        $rules = [
            'planId' => [$o->planId, SnippetRules::planId(...)],
            'surface' => [$surface, SnippetRules::surface(...)],
            'theme' => [$o->theme, SnippetRules::theme(...)],
            'locale' => [$o->locale, SnippetRules::locale(...)],
            'lingaraOrigin' => [$o->lingaraOrigin, SnippetRules::lingaraOrigin(...)],
            'title' => [$o->title, SnippetRules::title(...)],
            'id' => [$id, SnippetRules::id(...)],
            'tokenEndpoint' => [$o->tokenEndpoint, SnippetRules::tokenEndpoint(...)],
            'nonce' => [$o->nonce, SnippetRules::nonce(...)],
            'elementSrc' => [$o->elementSrc, SnippetRules::elementSrc(...)],
        ];
        foreach ($rules as $field => [$value, $ok]) {
            if ($value !== null && !$ok($value)) {
                throw new SnippetException($field);
            }
        }
    }

    /** An attribute value: the five characters, and nothing else. */
    private static function attr(string $value): string
    {
        return strtr($value, self::ATTRIBUTE);
    }

    /** A JSON string literal that can never close the script or open a comment. */
    private static function scriptString(string $value): string
    {
        return strtr(json_encode($value, self::JSON), self::SCRIPT);
    }
}
