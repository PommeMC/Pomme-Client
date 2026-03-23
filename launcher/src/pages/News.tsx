import { HiArrowLeft } from "react-icons/hi2";
import { useAppStateContext } from "../lib/state";
import { PatchNote } from "../lib/types";

export default function NewsPage({
  openPatchNote,
}: {
  openPatchNote: (note: PatchNote) => Promise<void>;
}) {
  const { selectedNote, setSelectedNote, news } = useAppStateContext();

  return (
    <div className="page news-page">
      {selectedNote ? (
        <div className="note-viewer">
          <button className="note-back" onClick={() => setSelectedNote(null)}>
            <HiArrowLeft /> Back
          </button>
          <h2 className="note-title">{selectedNote.title}</h2>
          <div className="note-body" dangerouslySetInnerHTML={{ __html: selectedNote.body }} />
        </div>
      ) : (
        <>
          <h2 className="news-page-heading">NEWS & UPDATES</h2>
          <div className="news-grid-full">
            {news.map((item) => (
              <div
                className="news-card-wide"
                key={item.version}
                onClick={() => openPatchNote(item)}
              >
                <div
                  className="news-card-img-wide"
                  style={{
                    backgroundImage: `url(${item.image_url})`,
                  }}
                >
                  <span className="news-type-badge">{item.entry_type}</span>
                </div>
                <div className="news-card-body-wide">
                  <span className="news-date">{item.date.replace(/-/g, ".")}</span>
                  <h3 className="news-title">{item.title}</h3>
                  <p className="news-desc-full">{item.summary}</p>
                  <span className="news-version">{item.version}</span>
                </div>
              </div>
            ))}
            {news.length === 0 && <p className="news-loading">Loading patch notes...</p>}
          </div>
        </>
      )}
    </div>
  );
}
