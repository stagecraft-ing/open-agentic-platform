#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0
/**
 * oap-ctl — remote control CLI for a running open-agentic-platform instance.
 *
 * Reads ~/.oap/control.port and ~/.oap/control.token, then calls the
 * control HTTP server exposed by the OAP desktop app.
 *
 * Usage:
 *   oap-ctl status
 *   oap-ctl projects
 *   oap-ctl sessions --project <id>
 *   oap-ctl messages --session <id> --project <project_id>
 *   oap-ctl send <prompt> --session <id> --project <project_id> [--model <m>]
 *   oap-ctl cancel --session <id>
 *   oap-ctl --help
 */

import { readFileSync, existsSync } from 'fs'
import { join } from 'path'
import { homedir } from 'os'
import { request } from 'http'

const OAP_DIR = join(homedir(), '.oap')
const PORT_FILE = join(OAP_DIR, 'control.port')
const TOKEN_FILE = join(OAP_DIR, 'control.token')

// ---------------------------------------------------------------------------
// Lockfile helpers
// ---------------------------------------------------------------------------

function readLockfiles() {
  if (!existsSync(PORT_FILE)) {
    console.error('The OAP desktop app does not appear to be running.')
    console.error(`Expected lockfile: ${PORT_FILE}`)
    console.error('Start the desktop app first, then retry.')
    process.exit(1)
  }
  const portRaw = readFileSync(PORT_FILE, 'utf8').trim()
  const port = parseInt(portRaw, 10)
  if (isNaN(port) || port < 1 || port > 65535) {
    console.error(`Invalid port in ${PORT_FILE}: ${portRaw}`)
    process.exit(1)
  }
  const token = existsSync(TOKEN_FILE) ? readFileSync(TOKEN_FILE, 'utf8').trim() : ''
  return { port, token }
}

// ---------------------------------------------------------------------------
// HTTP helper
// ---------------------------------------------------------------------------

function httpRequest(port, token, method, path, body) {
  return new Promise((resolve, reject) => {
    const payload = body != null ? JSON.stringify(body) : null
    const opts = {
      hostname: '127.0.0.1',
      port,
      path,
      method,
      headers: {
        'X-Control-Token': token,
        'Content-Type': 'application/json',
        ...(payload ? { 'Content-Length': Buffer.byteLength(payload) } : {}),
      },
    }
    const req = request(opts, (res) => {
      let data = ''
      res.on('data', (chunk) => { data += chunk })
      res.on('end', () => {
        if (res.statusCode === 401) {
          console.error('Authentication failed (stale token). Restart the desktop app and retry.')
          process.exit(1)
        }
        try {
          resolve({ status: res.statusCode, body: JSON.parse(data) })
        } catch {
          resolve({ status: res.statusCode, body: data })
        }
      })
    })
    req.on('error', (err) => {
      if (err.code === 'ECONNREFUSED') {
        console.error(`Connection refused on port ${port}. Is the OAP desktop app running?`)
        process.exit(1)
      }
      reject(err)
    })
    if (payload) req.write(payload)
    req.end()
  })
}

// ---------------------------------------------------------------------------
// Argument parser
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { flags: {}, positional: [] }
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith('--')) {
      const key = argv[i].slice(2)
      // Next element is the value unless it also starts with '--' or is missing.
      if (i + 1 < argv.length && !argv[i + 1].startsWith('--')) {
        args.flags[key] = argv[i + 1]
        i++
      } else {
        args.flags[key] = true
      }
    } else {
      args.positional.push(argv[i])
    }
  }
  return args
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function col(text, width) {
  const s = String(text ?? '')
  return s.length >= width ? s.slice(0, width - 1) + ' ' : s.padEnd(width)
}

function unwrap(res, label) {
  if (!res.body?.success) {
    const msg = res.body?.error ?? JSON.stringify(res.body)
    console.error(`${label} failed (HTTP ${res.status}): ${msg}`)
    process.exit(1)
  }
  return res.body.data
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

const HELP = `oap-ctl — remote control a running open-agentic-platform instance

Commands:
  status                                     Show server status and version
  projects                                   List all projects
  sessions --project <id>                    List sessions for a project
  messages --session <id> --project <id>     Show messages for a session
  send <prompt> --session <id> \\
               --project <id>               Send a prompt to a session (queued for execution)
  cancel --session <id>                      Cancel a running session
  --help, -h                                 Show this help

Environment:
  ~/.oap/control.port   Port written by the desktop app at startup
  ~/.oap/control.token  Auth token written by the desktop app at startup

Examples:
  oap-ctl status
  oap-ctl projects
  oap-ctl sessions --project my-project-id
  oap-ctl messages --session abc123 --project my-project-id`

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const argv = process.argv.slice(2)
  if (argv.length === 0 || argv[0] === '--help' || argv[0] === '-h') {
    console.log(HELP)
    process.exit(0)
  }

  const { port, token } = readLockfiles()
  const cmd = argv[0]
  const args = parseArgs(argv.slice(1))

  // ---- status ----
  if (cmd === 'status') {
    const res = await httpRequest(port, token, 'GET', '/control/status', null)
    const data = unwrap(res, 'status')
    console.log(`status:   ${data.status}`)
    console.log(`version:  ${data.version}`)
    console.log(`port:     ${port}`)
    return
  }

  // ---- projects ----
  if (cmd === 'projects') {
    const res = await httpRequest(port, token, 'GET', '/control/projects', null)
    const projects = unwrap(res, 'projects')
    if (!projects || projects.length === 0) {
      console.log('No projects found.')
      return
    }
    console.log(`${col('ID', 36)}  ${col('NAME', 32)}  PATH`)
    console.log('-'.repeat(100))
    for (const p of projects) {
      console.log(`${col(p.id, 36)}  ${col(p.name, 32)}  ${p.path ?? ''}`)
    }
    return
  }

  // ---- sessions ----
  if (cmd === 'sessions') {
    const projectId = args.flags.project
    if (!projectId) {
      console.error('Usage: oap-ctl sessions --project <id>')
      process.exit(1)
    }
    const res = await httpRequest(
      port, token, 'GET',
      `/control/projects/${encodeURIComponent(projectId)}/sessions`,
      null,
    )
    const sessions = unwrap(res, 'sessions')
    if (!sessions || sessions.length === 0) {
      console.log('No sessions found for this project.')
      return
    }
    console.log(`${col('ID', 36)}  ${col('MODEL', 28)}  LAST ACTIVITY`)
    console.log('-'.repeat(90))
    for (const s of sessions) {
      const ts = s.last_message_at
        ? new Date(s.last_message_at).toLocaleString()
        : '—'
      console.log(`${col(s.id, 36)}  ${col(s.model ?? '—', 28)}  ${ts}`)
    }
    return
  }

  // ---- messages ----
  if (cmd === 'messages') {
    const sessionId = args.flags.session
    const projectId = args.flags.project
    if (!sessionId || !projectId) {
      console.error('Usage: oap-ctl messages --session <id> --project <id>')
      process.exit(1)
    }
    const res = await httpRequest(
      port, token, 'GET',
      `/control/sessions/${encodeURIComponent(sessionId)}/messages/${encodeURIComponent(projectId)}`,
      null,
    )
    const history = unwrap(res, 'messages')
    if (!Array.isArray(history) || history.length === 0) {
      console.log('No messages found.')
      return
    }
    for (const msg of history) {
      // history entries are raw JSON from the JSONL file
      const role = (msg.type === 'human' ? 'user' : msg.type ?? 'unknown').padEnd(9)
      let text = ''
      if (typeof msg.message?.content === 'string') {
        text = msg.message.content
      } else if (Array.isArray(msg.message?.content)) {
        text = msg.message.content
          .filter((b) => b.type === 'text')
          .map((b) => b.text ?? '')
          .join('')
      } else if (typeof msg.content === 'string') {
        text = msg.content
      }
      console.log(`[${role}] ${text.slice(0, 140).replace(/\n/g, ' ')}`)
    }
    return
  }

  // ---- send ----
  if (cmd === 'send') {
    const sessionId = args.flags.session
    const projectId = args.flags.project
    const prompt = args.positional[0]
    if (!sessionId || !projectId || !prompt) {
      console.error('Usage: oap-ctl send <prompt> --session <id> --project <id>')
      process.exit(1)
    }
    const res = await httpRequest(
      port, token, 'POST',
      `/control/sessions/${encodeURIComponent(sessionId)}/messages`,
      { prompt, project_id: projectId },
    )
    const data = unwrap(res, 'send')
    console.log(JSON.stringify(data, null, 2))
    return
  }

  // ---- cancel ----
  if (cmd === 'cancel') {
    const sessionId = args.flags.session
    if (!sessionId) {
      console.error('Usage: oap-ctl cancel --session <id>')
      process.exit(1)
    }
    const res = await httpRequest(
      port, token, 'DELETE',
      `/control/sessions/${encodeURIComponent(sessionId)}`,
      null,
    )
    const data = unwrap(res, 'cancel')
    console.log(JSON.stringify(data, null, 2))
    return
  }

  console.error(`Unknown command: ${cmd}`)
  console.error('Run oap-ctl --help for usage.')
  process.exit(1)
}

main().catch((err) => {
  console.error('Fatal:', err.message ?? err)
  process.exit(1)
})
