import { HiPlay } from "react-icons/hi2";

export default function ServersPage() {
  return (
    <div className="page mock-page">
      <div className="mock-banner">This is a preview - functionality coming soon</div>
      <h2 className="mock-heading">SERVERS</h2>
      <div className="mock-list">
        {[
          { name: "Hypixel", ip: "mc.hypixel.net", players: "48,231", ping: "32ms", online: true },
          { name: "My SMP", ip: "play.mysmp.com", players: "12", ping: "8ms", online: true },
          { name: "Mineplex", ip: "us.mineplex.com", players: "3,891", ping: "45ms", online: true },
          { name: "Local Server", ip: "localhost:25565", players: "1", ping: "1ms", online: false },
        ].map((s) => (
          <div className="mock-server" key={s.ip}>
            <div className="mock-server-status">
              <div className={`mock-dot ${s.online ? "on" : "off"}`} />
            </div>
            <div className="mock-server-info">
              <span className="mock-server-name">{s.name}</span>
              <span className="mock-server-ip">{s.ip}</span>
            </div>
            <span className="mock-server-players">{s.players} players</span>
            <span className="mock-server-ping">{s.ping}</span>
            <button className="install-play-btn">
              <HiPlay /> Join
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
