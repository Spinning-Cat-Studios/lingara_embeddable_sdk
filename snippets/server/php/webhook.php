<?php

declare(strict_types=1);

namespace LingaraEmbedSnippets;

// lingara:begin webhook
use Lingara\Embed\PlayerDirectory;
use Lingara\Embed\PlayerRouter;
use Lingara\Embed\RoutedEvent;
use Lingara\Events\Generated\PracticeCompleted;
use Psr\Http\Message\ServerRequestInterface;
// lingara:end
// lingara:begin webhook
// POST /lingara/webhook, from any PSR-7 stack (or pass getallheaders()).
// Read the body exactly as received. handle() returns the status to send,
// with an empty body: 204, 401 a bad signature, 400 a malformed payload, 500
// your handler or directory threw. Lingara redelivers every non-2xx, and
// expects the 2xx within 10 s: enqueue slow work and return.
function webhookRoute(ServerRequestInterface $request, \PDO $db): int
{
    $router = new PlayerRouter(new PdoPlayerDirectory($db), getenv('LINGARA_WEBHOOK_SECRET') ?: '');
    return $router->handle((string) $request->getBody(), $request, static function (RoutedEvent $routed) use ($db): void {
        $event = $routed->event;
        if (!$event instanceof PracticeCompleted || $routed->playerRef === null) {
            return; // your own subject, a deleted player, or another type
        }
        // Delivery is at least once: deduplicate on the event's id.
        $insert = $db->prepare('INSERT INTO practice (event_id, player_id, plan_id) VALUES (?, ?, ?) ON CONFLICT (event_id) DO NOTHING')
            ?: throw new \RuntimeException('cannot prepare');
        $insert->execute([$event->id, $routed->playerRef, $event->data->getPlanId()]);
    });
}

// The directory, beside your players:
//   CREATE TABLE lingara_players (player_ref TEXT NOT NULL, subject TEXT NOT NULL);
//   CREATE UNIQUE INDEX ON lingara_players (subject);
//   CREATE INDEX ON lingara_players (player_ref);
final class PdoPlayerDirectory implements PlayerDirectory
{
    public function __construct(
        private readonly \PDO $db,
    ) {}

    public function remember(string $playerRef, string $subject): void
    {
        $this->run('INSERT INTO lingara_players (player_ref, subject) VALUES (?, ?) '
            . 'ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref', [$playerRef, $subject]);
    }

    public function playerRefFor(string $subject): ?string
    {
        $ref = $this->run('SELECT player_ref FROM lingara_players WHERE subject = ?', [$subject])->fetchColumn();
        return is_string($ref) ? $ref : null;
    }

    public function forget(string $playerRef): void
    {
        $this->run('DELETE FROM lingara_players WHERE player_ref = ?', [$playerRef]);
    }

    /** @param list<string> $args */
    private function run(string $sql, array $args): \PDOStatement
    {
        $statement = $this->db->prepare($sql) ?: throw new \RuntimeException('cannot prepare a statement');
        $statement->execute($args);
        return $statement;
    }
}
// lingara:end