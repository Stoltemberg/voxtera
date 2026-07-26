import { DOWNLOAD_URL } from "./download";

const features = [
  {
    number: "01",
    title: "Explore",
    text: "Atravesse vales, picos gelados e florestas antigas em um mundo que convida você a se perder de propósito.",
  },
  {
    number: "02",
    title: "Construa",
    text: "Transforme cada bloco em abrigo, cidade ou uma criação inteiramente sua — sozinho ou com amigos.",
  },
  {
    number: "03",
    title: "Aventure-se",
    text: "Descubra ruínas, encare criaturas e encontre novos caminhos além do horizonte.",
  },
];

const steps = [
  ["01", "Baixe o launcher", "Um único download para começar sua próxima aventura."],
  ["02", "Instale em poucos cliques", "O launcher cuida da instalação e das atualizações para você."],
  ["03", "Jogue", "Crie seu mundo e dê o primeiro passo quando estiver pronto."],
];

export function App() {
  return (
    <div className="site-shell">
      <header className="site-header">
        <a className="brand" href="#top" aria-label="Voxtera — início">
          <img src="/images/voxtera-logo.png" alt="Voxtera" />
        </a>
        <nav aria-label="Navegação principal">
          <a href="#game">O jogo</a>
          <a href="#start">Começar</a>
        </nav>
        <a className="header-download" href={DOWNLOAD_URL}>Baixar</a>
      </header>

      <main id="top">
        <section className="hero" aria-labelledby="hero-title">
          <img
            className="hero-image"
            src="/images/gameplay-capture.png"
            alt="Cordilheira voxel nevada com cavernas, árvores e antigas estruturas de pedra"
          />
          <div className="hero-legibility" aria-hidden="true" />
          <div className="hero-content page-width">
            <p className="eyebrow">Uma aventura feita de blocos</p>
            <h1 id="hero-title">Seu mundo voxel começa aqui</h1>
            <p className="hero-copy">Voxtera é um mundo aberto para explorar, construir e transformar cada descoberta em uma história sua.</p>
            <a className="button button-primary" href={DOWNLOAD_URL}>Baixar launcher para Windows (.exe)</a>
            <p className="platform-note">Windows 10/11 · instalação e atualizações automáticas</p>
          </div>
          <a className="scroll-cue" href="#game">Conheça o mundo <span aria-hidden="true">↓</span></a>
        </section>

        <section id="game" className="experience section page-width" aria-labelledby="game-title">
          <div className="section-intro">
            <p className="eyebrow">Um mundo, infinitas histórias</p>
            <h2 id="game-title">Feito para explorar</h2>
            <p>Todo vale esconde uma surpresa. Todo bloco pode ser o começo de algo maior.</p>
          </div>
          <div className="feature-grid">
            {features.map(({ number, title, text }) => (
              <article className="feature-card" key={title}>
                <span className="feature-number">{number}</span>
                <h3>{title}</h3>
                <p>{text}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="world-showcase" aria-label="Uma paisagem de Voxtera">
          <img src="/images/gameplay-capture.png" alt="Vista aérea de um vale de neve no mundo de Voxtera" />
          <div className="showcase-caption page-width">
            <p>Da primeira cabana à maior expedição, o próximo passo é seu.</p>
          </div>
        </section>

        <section id="start" className="start section page-width" aria-labelledby="start-title">
          <div className="section-intro">
            <p className="eyebrow">É simples começar</p>
            <h2 id="start-title">Comece em minutos</h2>
            <p>Sem complicação: o launcher prepara o caminho para você entrar no jogo.</p>
          </div>
          <ol className="steps">
            {steps.map(([number, title, text]) => (
              <li key={number}>
                <span>{number}</span>
                <div><h3>{title}</h3><p>{text}</p></div>
              </li>
            ))}
          </ol>
        </section>

        <section className="download-band" aria-labelledby="download-title">
          <div className="page-width download-band-content">
            <div>
              <p className="eyebrow">Pronto para partir?</p>
              <h2 id="download-title">Sua aventura começa agora.</h2>
            </div>
            <a className="button button-primary" href={DOWNLOAD_URL}>Baixar launcher para Windows (.exe)</a>
          </div>
        </section>
      </main>

      <footer className="site-footer page-width">
        <p>Voxtera · um mundo aberto, bloco por bloco.</p>
        <div>
          <a href="https://github.com/Stoltemberg/voxtera">GitHub</a>
          <a href="https://github.com/Stoltemberg/voxtera/releases">Histórico de lançamentos</a>
        </div>
      </footer>
    </div>
  );
}
