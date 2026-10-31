"use client";
import Image from "next/image";

import type { CatalogMetadata } from "../../lib/hiveway";

export const PublisherSummary = ({
  metadata,
}: {
  metadata?: CatalogMetadata | undefined;
}) => {
  if (!metadata) {
    return <span className="ws-muted">Publisher details loading…</span>;
  }
  const { publisher } = metadata;
  return (
    <span className="ws-publisher">
      {publisher.avatar && (
        <Image
          unoptimized
          src={publisher.avatar}
          alt=""
          width={24}
          height={24}
          loading="lazy"
          referrerPolicy="no-referrer"
        />
      )}
      <span>
        {publisher.username
          ? `@${publisher.username}`
          : (publisher.name ?? "Publisher profile unavailable")}
      </span>
      <span
        className={publisher.verified ? "ws-publisher-verified" : "ws-muted"}
      >
        {publisher.verified ? "Verified publisher" : "Unverified publisher"}
      </span>
      <span className="ws-muted">
        {publisher.total_tools} tools published · {metadata.invocation_count}{" "}
        invocations
      </span>
    </span>
  );
};
