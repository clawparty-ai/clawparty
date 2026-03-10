// onSend Filter: suppress-json
// If the reply text is valid JSON (and nothing else), suppress sending it.
// If the reply text is exactly "HEARTBEAT_OK", suppress sending it.
// Returns false to abort, null to continue.

export default function (ctx) {
  if (ctx.replyText === 'HEARTBEAT_OK') {
    console.info('[send-filter suppress-json] reply to', ctx.peer, 'is HEARTBEAT_OK, suppressing send')
    return false
  }
  try {
    JSON.parse(ctx.replyText)
    console.info('[send-filter suppress-json] reply to', ctx.peer, 'is pure JSON, suppressing send')
    return false
  } catch {
    console.info('[send-filter suppress-json] ok -> continue')
    return null
  }
}
