import { DOWNLOAD_URL } from "./download";

const features = [
  {
    number: "01",
    icon: "✦",
    title: "Explore",
    text: "Atravesse vales, picos gelados e florestas antigas em um mundo que convida você a se perder de propósito.",
    image: "/images/mountain-valley.jpg",
    alt: "Vale montanhoso e arborizado em Voxtera",
  },
  {
    number: "02",
    icon: "▦",
    title: "Construa",
    text: "Transforme cada bloco em abrigo, cidade ou uma criação inteiramente sua — sozinho ou com amigos.",
    image: "/images/gameplay-capture.png",
    alt: "Montanha voxel com cavernas e construções antigas",
  },
  {
    number: "03",
    icon: "⚔",
    title: "Aventure-se",
    text: "Descubra ruínas, encare criaturas e encontre novos caminhos além do horizonte.",
    image: "/images/ruins-adventure.jpg",
    alt: "Grupo em combate contra criaturas em ruínas vulcânicas de Voxtera",
  },
];

const steps = [
  ["/images/forest-dawn.jpg", "Floresta de Voxtera ao nascer do sol", "Baixe o launcher", "Um único download para começar sua próxima aventura."],
  ["/images/mountain-valley.jpg", "Vale voxel pronto para explorar", "Instale em poucos cliques", "O launcher cuida da instalação e das atualizações para você."],
  ["/images/ruins-adventure.jpg", "Aventureiros em uma ruína de Voxtera", "Jogue", "Crie seu mundo e dê o primeiro passo quando estiver pronto."],
];

export function App() {
  return (
    <div className="site-shell">
      <main id="top">
        <section className="hero" aria-labelledby="hero-title">
          <img className="hero-image" src="/images/forest-dawn.jpg" alt="Floresta voxel iluminada pelo nascer do sol" />
          <div className="hero-legibility" aria-hidden="true" />
          <header className="site-header page-width">
            <a className="brand" href="#top" aria-label="Voxtera — início">VOXTERA</a>
            <nav aria-label="Navegação principal">
              <a href="#game">O jogo</a>
              <a href="#start">Começar</a>
            </nav>
            <a className="header-download" href={DOWNLOAD_URL}>Baixar</a>
          </header>
          <div className="hero-content page-width">
            <p className="eyebrow">Uma aventura feita de blocos</p>
            <h1 id="hero-title">Seu mundo voxel começa aqui</h1>
            <p className="hero-copy">Voxtera é um mundo aberto para explorar, construir e transformar cada descoberta em uma história sua.</p>
            <a className="button button-primary" href={DOWNLOAD_URL}>Baixar launcher para Windows (.exe)</a>
            <p className="platform-note">Windows 10/11 · instalação e atualizações automáticas</p>
          </div>
          <a className="scroll-cue" href="#game">Conheça o mundo <span aria-hidden="true">↓</span></a>
        </section>

        <section id="game" className="game-intro section page-width" aria-labelledby="game-title">
          <p className="eyebrow">Um mundo, infinitas histórias</p>
          <h2 id="game-title">Feito para explorar</h2>
          <p>Todo vale esconde uma surpresa. Todo bloco pode ser o começo de algo maior.</p>
        </section>

        <section className="feature-rows" aria-label="Formas de jogar">
          {features.map(({ number, icon, title, text, image, alt }, index) => (
            <article className={`feature-row ${index % 2 ? "feature-row-reverse" : ""}`} key={title}>
              <div className="feature-image-wrap">
                <img src={image} alt={alt} />
              </div>
              <div className="feature-copy">
                <span className="feature-number">{number}</span>
                <span className="feature-icon" aria-hidden="true">{icon}</span>
                <h3>{title}</h3>
                <p>{text}</p>
              </div>
            </article>
          ))}
        </section>

        <section id="start" className="start section page-width" aria-labelledby="start-title">
          <div className="start-heading">
            <p className="eyebrow">É simples começar</p>
            <h2 id="start-title">Comece em minutos</h2>
            <p>Sem complicação: o launcher prepara o caminho para você entrar no jogo.</p>
          </div>
          <ol className="steps">
            {steps.map(([image, alt, title, text], index) => (
              <li key={title}>
                <span className="step-number">0{index + 1}</span>
                <div className="step-illustration"><img src={image} alt={alt} /></div>
                <h3>{title}</h3>
                <p>{text}</p>
              </li>
            ))}
          </ol>
        </section>

        <section className="download-band" aria-labelledby="download-title">
          <img src="/images/ruins-adventure.jpg" alt="" aria-hidden="true" />
          <div className="download-band-shade" aria-hidden="true" />
          <div className="page-width download-band-content">
            <div>
              <p className="eyebrow">Pronto para partir?</p>
              <h2 id="download-title">Sua aventura começa agora.</h2>
            </div>
            <a className="button button-primary" href={DOWNLOAD_URL}>Baixar launcher para Windows (.exe)</a>
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="page-width footer-grid">
          <div className="footer-brand"><a href="#top">VOXTERA</a><p>Um mundo aberto, bloco por bloco.</p></div>
          <div><h3>Explore</h3><a href="#game">O jogo</a><a href="#start">Começar</a></div>
          <div><h3>Comunidade</h3><a href="https://github.com/Stoltemberg/voxtera">GitHub</a><a href="https://github.com/Stoltemberg/voxtera/releases">Histórico de lançamentos</a></div>
          <div><h3>Download</h3><a href={DOWNLOAD_URL}>Baixar launcher</a><span>Windows 10/11</span></div>
        </div>
      </footer>
    </div>
  );
}
