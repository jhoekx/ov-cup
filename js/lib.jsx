import 'bootstrap';
import React, { useState, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import PropTypes from 'prop-types';

function CategorySelector({ selectedCategory, categories, onChange }) {
  const change = (e) => {
    onChange(e.target.value);
  };

  return (
    <form>
      <div className="mb-3">
        <label htmlFor="categorySelection" className="form-label">Leeftijdscategorie</label>
        <select className="form-select" id="categorySelection" defaultValue={selectedCategory} onChange={change}>
          <option label="Kies een categorie" />
          {categories.map((category) => <option key={category}>{category}</option>)}
        </select>
      </div>
    </form>
  );
}

CategorySelector.propTypes = {
  categories: PropTypes.arrayOf(PropTypes.string).isRequired,
  selectedCategory: PropTypes.string.isRequired,
  onChange: PropTypes.func.isRequired,
};

function RankingResult({ score, place, drop }) {
  return (
    <div className="col">
      {!drop && score}
      {drop && <del>{score}</del>}
      <br />
      <span className="text-muted">
        (
        {place}
        )
      </span>
    </div>
  );
}

RankingResult.propTypes = {
  score: PropTypes.number,
  place: PropTypes.oneOfType([PropTypes.number, PropTypes.string]),
  drop: PropTypes.bool,
};

RankingResult.defaultProps = {
  score: 0,
  place: '-',
  drop: false,
};

function RankingEntry({ entry }) {
  return (
    <div className="row mt-lg-2 mt-3 pb-2">
      <div className="col-lg-5">
        <div className="row gx-3">
          <div className="col-2 text-end">
            {entry.place}
          </div>
          <div className="col-10">
            {entry.name}
            {' '}
            <br />
            {' '}
            <span className="text-muted">{entry.club}</span>
          </div>
        </div>
      </div>
      <div className="col-lg mt-1 mt-lg-0 text-end">
        <div className="row">
          {entry.scores.map(
            (result) => (
              <RankingResult
                key={result.eventId}
                score={result.score || undefined}
                place={result.place || undefined}
                drop={result.drop || false}
              />
            ),
          )}
          <div className="col">
            <strong>{entry.totalScore}</strong>
          </div>
        </div>
      </div>
    </div>
  );
}

RankingEntry.propTypes = {
  entry: PropTypes.shape({
    place: PropTypes.string.isRequired,
    name: PropTypes.string.isRequired,
    club: PropTypes.string.isRequired,
    totalScore: PropTypes.number.isRequired,
    scores: PropTypes.arrayOf(PropTypes.shape({
      eventId: PropTypes.number.isRequired,
      score: PropTypes.number,
      place: PropTypes.number,
      drop: PropTypes.bool,
    })),
  }).isRequired,
};

function Ranking({
  categories, cup, season, events,
}) {
  const [selectedCategory, setSelectedCategory] = useState('');
  const [isLoading, setLoading] = useState(false);
  const [ranking, setRanking] = useState([]);

  const generateRanking = (data) => {
    const entries = [];
    let place = 0;
    let previousScore = 5001;
    data.forEach((entry) => {
      place += 1;
      let thisPlace;
      if (entry.totalScore < previousScore) {
        thisPlace = `${place}.`;
      } else {
        thisPlace = '-';
      }
      previousScore = entry.totalScore;

      const nonZeroResults = entry.scores
        .map((result) => result.score)
        .filter((result) => result > 0)
        .sort((x, y) => y - x)
        .slice(0, events);
      const results = entry.scores.map((result) => {
        let drop = false;
        if (!nonZeroResults.includes(result.score) && result.score > 0) {
          drop = true;
        }
        return { ...result, drop };
      });
      entries.push({ ...entry, scores: results, place: thisPlace });
    });
    setRanking(entries);
  };

  useEffect(() => {
    if (!selectedCategory) {
      setRanking([]);
      return;
    }
    setLoading(true);
    const url = new URL('./cgi-bin/cup-cgi', window.location);
    url.searchParams.set('cup', cup);
    url.searchParams.set('season', season);
    url.searchParams.set('ageClass', selectedCategory);
    url.searchParams.set('events', events);

    fetch(url)
      .then((response) => response.json())
      .then((data) => { setLoading(false); return data; })
      .then(generateRanking);
  }, [selectedCategory]);

  return (
    <>
      <CategorySelector
        categories={categories}
        selectedCategory={selectedCategory}
        onChange={setSelectedCategory}
      />
      {ranking.map((entry) => <RankingEntry key={`${entry.name}${entry.club}`} entry={entry} />)}
      {isLoading && (
        <div className="text-center">
          <div className="spinner-border" role="status">
            <span className="visually-hidden">Loading...</span>
          </div>
        </div>
      )}
      {selectedCategory
        && !isLoading
        && ranking.length === 0
        && (
          <div className="alert alert-info">
            Geen resultaten voor
            {` ${selectedCategory}`}
            .
          </div>
        )}
    </>
  );
}

Ranking.propTypes = {
  categories: PropTypes.arrayOf(PropTypes.string).isRequired,
  cup: PropTypes.string.isRequired,
  season: PropTypes.string.isRequired,
  events: PropTypes.number.isRequired,
};

const categories = [];
document.querySelectorAll('#ranking span').forEach((category) => {
  categories.push(category.textContent);
});

const rankingContainer = document.getElementById('ranking');
if (rankingContainer) {
  const root = createRoot(rankingContainer);
  root.render(
    <Ranking
      categories={categories}
      cup={rankingContainer.dataset.cup}
      season={rankingContainer.dataset.season}
      events={parseFloat(rankingContainer.dataset.events)}
    />,
    rankingContainer,
  );
}
