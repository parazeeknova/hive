"use client";
import { useQuery } from "@tanstack/react-query";
import Image from "next/image";
import Link from "next/link";

import { Icon } from "../../components/icon";
import { errorMessage, hiveway } from "../../lib/hiveway";
import type { McpIntegration } from "../../lib/hiveway";

interface Connection {
  id: string;
  name: string;
  online: boolean;
  tools: unknown[];
}
interface SharedIntegration {
  name: string;
  online: boolean;
  tools: number;
}
interface ConnectionStatus {
  connections: Connection[];
  shared: SharedIntegration[];
}
/** Prefer the caller's own connection; otherwise report Hive's shared one. */
const availability = (
  status: ConnectionStatus | undefined,
  slug: string
): { online: boolean; tools: number; shared: boolean } | undefined => {
  const own = status?.connections.find((item) => item.name === slug);
  if (own) {
    return { online: own.online, shared: false, tools: own.tools.length };
  }
  const shared = status?.shared.find((item) => item.name === slug);
  return shared && { online: shared.online, shared: true, tools: shared.tools };
};
const statusLabel = (
  connected: ReturnType<typeof availability>,
  shared: boolean
) => {
  if (connected?.online) {
    return connected.shared
      ? `Available to every Hive account · ${connected.tools} read-only tools`
      : `Connected · ${connected.tools} tools`;
  }
  return shared ? "Shared integration temporarily offline" : "Ready to connect";
};
export const IntegrationPanel = ({
  integration,
  token,
}: {
  integration: McpIntegration;
  token: string;
}) => {
  const connections = useQuery({
    enabled: Boolean(token),
    queryFn: ({ signal }) =>
      hiveway<ConnectionStatus>("/v1/connections", token, { signal }),
    queryKey: ["hive", token, "connections"],
    refetchInterval: 10_000,
  });
  const connected = availability(connections.data, integration.slug);
  const shared = Boolean(integration.shared_owner);
  const local = connections.data?.connections.find(
    (connection) => connection.name === "local-preview" && connection.online
  );
  return (
    <section
      className="mt-5 space-y-4 text-xs leading-relaxed text-[#626d56]"
      aria-label={`${integration.name} integration`}
    >
      <div className="rounded-lg border border-[#dfe5d7] bg-[#f5f8ef] p-3">
        <p className="flex items-center gap-2 font-semibold text-[#33482b]">
          <Icon name={connected?.online ? "check" : "help"} size={15} />
          {statusLabel(connected, shared)}
        </p>
        <p className="mt-1">
          {integration.byok
            ? "Use your own upstream API key. Your usage stays with your provider account; Hive does not share another user's credentials."
            : "Search real templates, compare image previews, then choose the design to build. No setup or API key is needed."}
        </p>
        {connections.error && (
          <p role="alert">{errorMessage(connections.error)}</p>
        )}
      </div>
      {integration.previews.length > 0 && (
        <div>
          <h3 className="mb-2 font-semibold text-[#39482f]">
            Template previews
          </h3>
          <div className="grid grid-cols-2 gap-2">
            {integration.previews.map((preview) => (
              <a
                key={preview.slug}
                href={preview.src}
                target="_blank"
                rel="noopener noreferrer"
                className="overflow-hidden rounded-lg border border-[#e0e4da] bg-white"
              >
                <Image
                  unoptimized
                  src={preview.src}
                  alt={preview.title}
                  width={480}
                  height={300}
                  className="h-28 w-full object-cover object-top"
                  loading="lazy"
                />
                <span className="block p-2 text-[11px]">{preview.title}</span>
              </a>
            ))}
          </div>
          <p className="mt-2">
            Open an image to inspect the full design. For clickable HTML, ask
            your agent to preview the selected templates locally.
          </p>
        </div>
      )}
      {integration.slug === "templr" && (
        <div className="rounded-lg border border-[#e0e4da] p-3">
          <h3 className="font-semibold text-[#39482f]">
            Preview on your computer
          </h3>
          <p className="mt-1">
            {local
              ? "Hive Local is connected. Your agent can download the selected HTML and start a local preview server."
              : "Enable Hive Local in your agent to download the HTML and open a server on your own computer."}
          </p>
          <p className="mt-2 rounded bg-[#f4f5ef] p-2">
            Ask: “Find clothing brand templates in Hive, show the real previews,
            then preview my choices locally.”
          </p>
        </div>
      )}
      <details className="rounded-lg border border-[#e0e4da] p-3">
        <summary className="cursor-pointer font-semibold">
          Available tools · {integration.tools.length}
        </summary>
        <ul className="mt-2 space-y-1">
          {integration.tools.map((name) => (
            <li key={name}>
              <code>{name}</code>
            </li>
          ))}
        </ul>
      </details>
      <Link
        href="/docs#docs-hive-local"
        className="inline-flex items-center gap-1 font-semibold text-[#496337] underline underline-offset-4"
      >
        {local ? "Manage Hive Local setup" : "Set up Hive Local"}
        <Icon name="external" size={12} />
      </Link>
      <p className="text-[10px]">
        Integration published and maintained by Hive. Upstream tools and content
        remain the responsibility of their respective providers.
      </p>
    </section>
  );
};
