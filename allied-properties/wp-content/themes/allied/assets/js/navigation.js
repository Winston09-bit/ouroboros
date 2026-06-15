/* Allied Properties — front-end behaviour (vanilla, no dependencies) */
(function () {
  "use strict";

  // Mobile navigation toggle
  var toggle = document.querySelector(".nav-toggle");
  var nav = document.getElementById("primary-nav");
  if (toggle && nav) {
    toggle.addEventListener("click", function () {
      var open = nav.classList.toggle("is-open");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    });
    // close on Escape
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && nav.classList.contains("is-open")) {
        nav.classList.remove("is-open");
        toggle.setAttribute("aria-expanded", "false");
        toggle.focus();
      }
    });
  }

  // Portfolio filter (client-side, progressive enhancement)
  var filterLinks = document.querySelectorAll("[data-filter]");
  var items = document.querySelectorAll("[data-cats]");
  if (filterLinks.length && items.length) {
    filterLinks.forEach(function (link) {
      link.addEventListener("click", function (e) {
        e.preventDefault();
        var term = link.getAttribute("data-filter");
        filterLinks.forEach(function (l) { l.classList.remove("is-active"); });
        link.classList.add("is-active");
        items.forEach(function (item) {
          var cats = (item.getAttribute("data-cats") || "").split(" ");
          item.style.display = term === "all" || cats.indexOf(term) !== -1 ? "" : "none";
        });
      });
    });
  }
})();
