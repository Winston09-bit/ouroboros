/* Allied Properties — front-end behaviour (vanilla, no dependencies) */
(function () {
  "use strict";

  /* ---- Mobile navigation ---- */
  var toggle = document.querySelector(".nav-toggle");
  var nav = document.getElementById("primary-nav");

  function setNav(open) {
    nav.classList.toggle("is-open", open);
    toggle.setAttribute("aria-expanded", open ? "true" : "false");
    document.body.classList.toggle("nav-open", open);
  }

  if (toggle && nav) {
    toggle.addEventListener("click", function () {
      setNav(!nav.classList.contains("is-open"));
    });

    // Close when a menu link is tapped (so navigation feels immediate).
    nav.addEventListener("click", function (e) {
      if (e.target.closest("a")) { setNav(false); }
    });

    // Close on Escape and return focus to the toggle.
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && nav.classList.contains("is-open")) {
        setNav(false);
        toggle.focus();
      }
    });

    // Reset state if resized back up to desktop.
    var mq = window.matchMedia("(min-width: 981px)");
    (mq.addEventListener ? mq.addEventListener.bind(mq, "change") : mq.addListener.bind(mq))(function () {
      if (mq.matches) { setNav(false); }
    });
  }

  /* ---- Portfolio filter (progressive enhancement) ---- */
  var filterLinks = document.querySelectorAll("[data-filter]");
  var items = document.querySelectorAll("[data-cats]");
  if (filterLinks.length && items.length) {
    filterLinks.forEach(function (link) {
      link.setAttribute("aria-pressed", link.classList.contains("is-active") ? "true" : "false");
      link.addEventListener("click", function (e) {
        e.preventDefault();
        var term = link.getAttribute("data-filter");
        filterLinks.forEach(function (l) {
          l.classList.remove("is-active");
          l.setAttribute("aria-pressed", "false");
        });
        link.classList.add("is-active");
        link.setAttribute("aria-pressed", "true");
        items.forEach(function (item) {
          var cats = (item.getAttribute("data-cats") || "").split(" ");
          item.hidden = !(term === "all" || cats.indexOf(term) !== -1);
        });
      });
    });
  }

  /* ---- Image load-in: fade media in once decoded (no layout shift) ---- */
  document.querySelectorAll(".card__media img, .hero__media img").forEach(function (img) {
    img.classList.add("allied-fade");
    if (img.complete && img.naturalWidth) {
      img.classList.add("is-loaded");
    } else {
      img.addEventListener("load", function () { img.classList.add("is-loaded"); }, { once: true });
      img.addEventListener("error", function () { img.classList.add("is-loaded"); }, { once: true });
    }
  });
})();
