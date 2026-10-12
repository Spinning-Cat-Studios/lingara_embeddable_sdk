<?php

declare(strict_types=1);

namespace LingaraEmbedSnippets;

// lingara:begin deletePlayer
use Lingara\Embed\EmbedServer;
// lingara:end
// lingara:begin deletePlayer
// From your own account-deletion path: revokes the player's tokens at
// Lingara, then forgets their subjects. An unknown player is a success too,
// so a retry is safe; on a failure the rows are kept.
function deleteAccount(EmbedServer $embed, \PDO $db, string $playerId): void
{
    $embed->deletePlayer($playerId);
    $delete = $db->prepare('DELETE FROM players WHERE id = ?') ?: throw new \RuntimeException('cannot prepare');
    $delete->execute([$playerId]);
}
// lingara:end