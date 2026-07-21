import voxteraLogo from '../voxtera_logo.png';

export function App() {
  return (
    <main className="launcher-shell">
      <section className="launcher-card" aria-labelledby="launcher-title">
        <img className="launcher-logo" src={voxteraLogo} alt="" />
        <p className="launcher-eyebrow">Launcher oficial</p>
        <h1 id="launcher-title">Voxtera</h1>
        <p className="launcher-description">
          Prepare sua instalação para entrar no mundo de Voxtera.
        </p>
        <button type="button">Verificar instalação</button>
      </section>
    </main>
  );
}
