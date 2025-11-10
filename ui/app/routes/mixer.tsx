import { useState, useCallback } from "react";
import { Layout } from "~/components/layout/Layout";
import { Track } from "~/components/mixer/Track";
import { mockTracks, mockMasterTrack } from "~/lib/mockData";
import type { Track as TrackType } from "~/types/audio";

export default function MixerPage() {
  const [tracks, setTracks] = useState<TrackType[]>(mockTracks);
  const [masterTrack, setMasterTrack] = useState<TrackType>(mockMasterTrack);

  const updateTrack = useCallback((trackId: string, updates: Partial<TrackType>) => {
    setTracks((prev) =>
      prev.map((track) =>
        track.id === trackId ? { ...track, ...updates } : track
      )
    );
  }, []);

  const updateMaster = useCallback((_trackId: string, updates: Partial<TrackType>) => {
    setMasterTrack((prev) => ({ ...prev, ...updates }));
  }, []);

  return (
    <Layout>
      <div className="h-full bg-zinc-900 p-6">
        <div className="mb-6">
          <h2 className="text-2xl font-bold text-white">Mixer</h2>
          <p className="text-zinc-400 mt-1">
            8-track mixer with effects, pan, and volume controls
          </p>
        </div>

        <div className="flex gap-4 overflow-x-auto pb-4">
          {/* Regular Tracks */}
          {tracks.map((track) => (
            <Track
              key={track.id}
              track={track}
              onUpdate={updateTrack}
            />
          ))}

          {/* Divider */}
          <div className="w-px bg-zinc-700 self-stretch mx-2" />

          {/* Master Track */}
          <Track
            track={masterTrack}
            onUpdate={updateMaster}
            isMaster
          />
        </div>
      </div>
    </Layout>
  );
}
