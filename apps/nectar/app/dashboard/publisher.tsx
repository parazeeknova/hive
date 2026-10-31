"use client";
import Image from "next/image";

import type { CatalogMetadata } from "../../lib/hiveway";

export const PublisherSummary = ({
  metadata,
}: {
  metadata?: CatalogMetadata | undefined;
}) => {
  if (!metadata) {
    return (
      <span className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
        Publisher details loading…
      </span>
    );
  }
  const { publisher } = metadata;
  return (
    <span className="ws-publisher inline-flex flex-wrap items-center gap-[6px_10px] text-[11px] leading-[1.6] [&_img]:rounded-full [&_img]:object-cover">
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
        className={
          publisher.verified
            ? "ws-publisher-verified text-[color:#437427]"
            : "ws-muted text-[11px] leading-[1.85] text-[color:#686f60]"
        }
      >
        {publisher.verified ? "Verified publisher" : "Unverified publisher"}
      </span>
      <span className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
        {publisher.total_tools} tools published · {metadata.invocation_count}{" "}
        invocations
      </span>
    </span>
  );
};
